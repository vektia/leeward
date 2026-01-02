//! Sandbox worker process management

use crate::{pipe::ParentPipe, ExecutionResult, LeewardError, Result, SandboxConfig};
use std::os::unix::io::RawFd;

/// State of a worker in the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Ready to accept work
    Idle,
    /// Currently executing code
    Busy,
    /// Being recycled (killed and respawned)
    Recycling,
    /// Dead/failed
    Dead,
}

/// A pre-forked sandboxed worker process
///
/// In the new paradigm:
/// - Workers are created at daemon startup with clone3 + CLONE_INTO_CGROUP
/// - Python interpreter is already loaded and idle
/// - Code is sent via pipe, execution is immediate (~0.5ms)
/// - Workers survive denied syscalls (SECCOMP_USER_NOTIF)
#[derive(Debug)]
pub struct Worker {
    /// Unique worker ID
    pub id: u32,
    /// Current state
    pub state: WorkerState,
    /// Process ID (if running)
    pub pid: Option<i32>,
    /// Number of executions completed
    pub execution_count: u64,
    /// Configuration for this worker
    config: SandboxConfig,
    /// Communication pipe with the worker
    pipe: Option<ParentPipe>,
    /// Cgroup file descriptor (for CLONE_INTO_CGROUP)
    cgroup_fd: Option<RawFd>,
}

impl Worker {
    /// Create a new worker with the given config
    pub fn new(id: u32, config: SandboxConfig) -> Self {
        Self {
            id,
            state: WorkerState::Dead,
            pid: None,
            execution_count: 0,
            config,
            pipe: None,
            cgroup_fd: None,
        }
    }

    /// Spawn the worker process using pre-fork model
    ///
    /// This uses clone3 with CLONE_INTO_CGROUP to create a fully isolated
    /// worker process with Python already loaded.
    pub fn spawn(&mut self) -> Result<()> {
        use crate::isolation::clone3;
        use crate::pipe::WorkerPipe;

        tracing::info!(worker_id = self.id, "spawning pre-forked worker");

        // Create communication pipes
        let worker_pipe = WorkerPipe::new()?;
        let (parent_pipe, child_pipe) = worker_pipe.split();

        // TODO: Create cgroup for this worker and get fd
        // For now, use -1 as placeholder (will be implemented with cgroups)
        let cgroup_fd = -1;

        // Get namespace flags from config
        let namespace_flags = self.config_to_namespace_flags();

        // Clone config for child process
        let config = self.config.clone();

        // Clone the worker process with full isolation
        let pid = clone3::clone_worker(cgroup_fd, namespace_flags, move || {
            // Child process: Set up isolation and load Python
            worker_main(child_pipe, &config)
        })?;

        // Parent process: Store worker info
        self.pid = Some(pid);
        self.pipe = Some(parent_pipe);
        self.cgroup_fd = Some(cgroup_fd);
        self.state = WorkerState::Idle;

        tracing::info!(
            worker_id = self.id,
            pid = pid,
            "worker spawned and ready"
        );

        Ok(())
    }

    /// Execute code in this worker via pipe
    pub fn execute(&mut self, code: &str) -> Result<ExecutionResult> {
        if self.state != WorkerState::Idle {
            return Err(LeewardError::Execution(format!(
                "worker {} is not idle (state: {:?})",
                self.id, self.state
            )));
        }

        let pipe = self
            .pipe
            .as_mut()
            .ok_or_else(|| LeewardError::Execution("worker pipe not initialized".into()))?;

        self.state = WorkerState::Busy;
        tracing::debug!(worker_id = self.id, code_len = code.len(), "sending code to worker");

        // Send code via pipe
        pipe.send_code(code.as_bytes())?;

        // Receive result via pipe
        let _result_bytes = pipe.recv_result()?;

        // TODO: Deserialize result from MessagePack
        let result = ExecutionResult::default();

        self.execution_count += 1;
        self.state = WorkerState::Idle;

        tracing::debug!(
            worker_id = self.id,
            execution_count = self.execution_count,
            "execution completed"
        );

        Ok(result)
    }

    /// Kill and recycle this worker
    pub fn recycle(&mut self) -> Result<()> {
        tracing::info!(worker_id = self.id, "recycling worker");
        self.state = WorkerState::Recycling;

        // Kill existing process if any
        if let Some(pid) = self.pid {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }

        // Close pipe
        self.pipe = None;

        // Reset state
        self.pid = None;
        self.execution_count = 0;

        // Spawn new worker
        self.spawn()
    }

    /// Check if worker should be recycled based on execution count
    #[must_use]
    pub fn should_recycle(&self, max_executions: u64) -> bool {
        self.execution_count >= max_executions
    }

    /// Convert config to namespace flags
    fn config_to_namespace_flags(&self) -> u64 {
        use nix::sched::CloneFlags;

        let mut flags = CloneFlags::CLONE_NEWUSER
            | CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWIPC
            | CloneFlags::CLONE_NEWUTS;

        if !self.config.allow_network {
            flags |= CloneFlags::CLONE_NEWNET;
        }

        flags.bits() as u64
    }
}

/// Worker main function (runs in child process)
fn worker_main(mut pipe: crate::pipe::ChildPipe, _config: &SandboxConfig) -> Result<()> {
    use crate::isolation::{LandlockConfig, SeccompConfig};

    tracing::debug!("worker process starting, setting up isolation");

    // Set up Landlock filesystem restrictions
    LandlockConfig::default().apply()?;

    // Set up seccomp with NOTIFY mode
    let _notify_fd = SeccompConfig::default().apply()?;

    // TODO: Load Python interpreter
    tracing::info!("worker isolation complete, loading Python");

    // Enter idle loop, waiting for code via pipe
    loop {
        tracing::debug!("worker waiting for code");

        // Receive code from daemon
        let code = match pipe.recv_code() {
            Ok(code) => code,
            Err(e) => {
                tracing::error!("failed to receive code: {}", e);
                break;
            }
        };

        tracing::debug!(code_len = code.len(), "received code, executing");

        // TODO: Execute code in Python
        // For now, just echo back
        let result = code;

        // Send result back to daemon
        if let Err(e) = pipe.send_result(&result) {
            tracing::error!("failed to send result: {}", e);
            break;
        }

        tracing::debug!("result sent, waiting for next code");
    }

    Ok(())
}
