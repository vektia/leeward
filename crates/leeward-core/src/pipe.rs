//! Pipe-based communication for worker code execution

use crate::{LeewardError, Result};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::io::{Read, Write};

/// A pair of pipes for bidirectional communication with a worker
#[derive(Debug)]
pub struct WorkerPipe {
    /// Pipe for sending code to worker (daemon writes, worker reads)
    pub code_tx: std::fs::File,
    pub code_rx: std::fs::File,

    /// Pipe for receiving results from worker (worker writes, daemon reads)
    pub result_tx: std::fs::File,
    pub result_rx: std::fs::File,
}

impl WorkerPipe {
    /// Create a new pair of pipes
    pub fn new() -> Result<Self> {
        let (code_rx, code_tx) = create_pipe()?;
        let (result_rx, result_tx) = create_pipe()?;

        Ok(Self {
            code_tx,
            code_rx,
            result_tx,
            result_rx,
        })
    }

    /// Split the pipe into parent and child ends
    pub fn split(self) -> (ParentPipe, ChildPipe) {
        let parent = ParentPipe {
            code_tx: self.code_tx,
            result_rx: self.result_rx,
        };

        let child = ChildPipe {
            code_rx: self.code_rx,
            result_tx: self.result_tx,
        };

        (parent, child)
    }
}

/// Parent end of the worker pipe (daemon side)
#[derive(Debug)]
pub struct ParentPipe {
    /// Write code to worker
    code_tx: std::fs::File,
    /// Read results from worker
    result_rx: std::fs::File,
}

impl ParentPipe {
    /// Send code to the worker
    pub fn send_code(&mut self, code: &[u8]) -> Result<()> {
        // Send length prefix (4 bytes, big-endian)
        let len_bytes = (code.len() as u32).to_be_bytes();
        self.code_tx.write_all(&len_bytes)?;

        // Send code
        self.code_tx.write_all(code)?;

        self.code_tx.flush()?;

        Ok(())
    }

    /// Receive result from the worker
    pub fn recv_result(&mut self) -> Result<Vec<u8>> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        self.result_rx.read_exact(&mut len_bytes)?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > 10 * 1024 * 1024 {
            return Err(LeewardError::Execution(format!(
                "result too large: {} bytes",
                len
            )));
        }

        // Read result
        let mut result = vec![0u8; len];
        self.result_rx.read_exact(&mut result)?;

        Ok(result)
    }

    /// Get raw file descriptor for code transmission (for io_uring)
    pub fn code_tx_fd(&self) -> RawFd {
        self.code_tx.as_raw_fd()
    }

    /// Get raw file descriptor for result reception (for io_uring)
    pub fn result_rx_fd(&self) -> RawFd {
        self.result_rx.as_raw_fd()
    }
}

/// Child end of the worker pipe (worker side)
#[derive(Debug)]
pub struct ChildPipe {
    /// Read code from daemon
    code_rx: std::fs::File,
    /// Write results to daemon
    result_tx: std::fs::File,
}

impl ChildPipe {
    /// Wait for and receive code from daemon
    pub fn recv_code(&mut self) -> Result<Vec<u8>> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        self.code_rx.read_exact(&mut len_bytes)?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > 1024 * 1024 {
            return Err(LeewardError::Execution(format!(
                "code too large: {} bytes",
                len
            )));
        }

        // Read code
        let mut code = vec![0u8; len];
        self.code_rx.read_exact(&mut code)?;

        Ok(code)
    }

    /// Send result back to daemon
    pub fn send_result(&mut self, result: &[u8]) -> Result<()> {
        // Send length prefix
        let len_bytes = (result.len() as u32).to_be_bytes();
        self.result_tx.write_all(&len_bytes)?;

        // Send result
        self.result_tx.write_all(result)?;

        self.result_tx.flush()?;

        Ok(())
    }

    /// Get raw file descriptors (for passing to child process)
    pub fn into_raw_fds(self) -> (RawFd, RawFd) {
        use std::os::unix::io::IntoRawFd;
        (self.code_rx.into_raw_fd(), self.result_tx.into_raw_fd())
    }
}

/// Create a pipe (returns read end, write end)
fn create_pipe() -> Result<(std::fs::File, std::fs::File)> {
    let mut fds = [0i32; 2];

    // SAFETY: pipe2 syscall
    let ret = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };

    if ret != 0 {
        return Err(LeewardError::Io(std::io::Error::last_os_error()));
    }

    // SAFETY: We just created these file descriptors
    let read_end = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let write_end = unsafe { std::fs::File::from_raw_fd(fds[1]) };

    Ok((read_end, write_end))
}
