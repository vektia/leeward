//! Seccomp-BPF syscall filtering with SECCOMP_USER_NOTIF support

use crate::{LeewardError, Result};
use std::os::unix::io::{AsRawFd, RawFd};

/// Configuration for seccomp filtering
#[derive(Debug, Clone)]
pub struct SeccompConfig {
    /// Use NOTIFY mode instead of KILL (allows supervisor intervention)
    pub notify_mode: bool,
    /// Syscalls to allow
    pub allowed_syscalls: Vec<i64>,
    /// Log denied syscalls before killing
    pub log_denials: bool,
}

impl Default for SeccompConfig {
    fn default() -> Self {
        Self {
            notify_mode: true,
            allowed_syscalls: default_python_syscalls(),
            log_denials: true,
        }
    }
}

impl SeccompConfig {
    /// Apply the seccomp filter to the current process
    ///
    /// If notify_mode is true, returns a file descriptor for receiving
    /// seccomp notifications. The supervisor can poll this fd and decide
    /// what to do with blocked syscalls.
    pub fn apply(&self) -> Result<Option<SeccompNotifyFd>> {
        // TODO: Build and apply BPF filter using seccompiler
        // For notify mode:
        // 1. Build filter with SECCOMP_RET_USER_NOTIF for blocked syscalls
        // 2. Use seccomp(SECCOMP_SET_MODE_FILTER, ...) to apply
        // 3. Get notification fd from seccomp(SECCOMP_GET_NOTIF_SIZES, ...)

        tracing::debug!(
            notify = self.notify_mode,
            syscalls = self.allowed_syscalls.len(),
            "applying seccomp filter"
        );

        if self.notify_mode {
            // TODO: Return actual notification fd
            // For now, return None as placeholder
            Ok(None)
        } else {
            Ok(None)
        }
    }
}

/// File descriptor for receiving seccomp notifications
///
/// When a process attempts a blocked syscall with SECCOMP_RET_USER_NOTIF,
/// the kernel sends a notification to this fd. The supervisor can:
/// - Return EACCES to continue execution
/// - Return different error code
/// - Allow the syscall (with caution)
/// - Terminate the process
#[derive(Debug)]
pub struct SeccompNotifyFd {
    fd: RawFd,
}

impl SeccompNotifyFd {
    /// Create from raw file descriptor
    pub fn from_raw_fd(fd: RawFd) -> Self {
        Self { fd }
    }

    /// Get the raw file descriptor
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Wait for a seccomp notification
    pub fn wait_notification(&self) -> Result<SeccompNotification> {
        // TODO: Read seccomp_notif structure from fd
        // struct seccomp_notif {
        //     __u64 id;
        //     __u32 pid;
        //     __u32 flags;
        //     struct seccomp_data data;
        // };

        tracing::debug!("waiting for seccomp notification");

        // Placeholder
        Ok(SeccompNotification {
            id: 0,
            pid: 0,
            syscall: 0,
            args: [0; 6],
        })
    }

    /// Send a response to a seccomp notification
    pub fn send_response(&self, notif: &SeccompNotification, response: SeccompResponse) -> Result<()> {
        // TODO: Write seccomp_notif_resp structure to fd
        // struct seccomp_notif_resp {
        //     __u64 id;
        //     __s64 val;  // Return value or error code
        //     __s32 error;  // errno value
        //     __u32 flags;
        // };

        tracing::debug!(
            id = notif.id,
            pid = notif.pid,
            syscall = notif.syscall,
            response = ?response,
            "sending seccomp response"
        );

        Ok(())
    }
}

/// A seccomp notification from the kernel
#[derive(Debug, Clone)]
pub struct SeccompNotification {
    /// Notification ID (must be included in response)
    pub id: u64,
    /// Process ID that triggered the notification
    pub pid: u32,
    /// Syscall number
    pub syscall: i64,
    /// Syscall arguments
    pub args: [u64; 6],
}

/// Response to a seccomp notification
#[derive(Debug, Clone)]
pub enum SeccompResponse {
    /// Deny with EACCES
    DenyWithEacces,
    /// Deny with custom error
    DenyWithError(i32),
    /// Allow the syscall (use with extreme caution)
    Allow,
    /// Continue with specific return value
    ContinueWithValue(i64),
}

impl Drop for SeccompNotifyFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Default syscalls needed for Python to run
fn default_python_syscalls() -> Vec<i64> {
    vec![
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_close,
        libc::SYS_fstat,
        libc::SYS_lseek,
        libc::SYS_mmap,
        libc::SYS_mprotect,
        libc::SYS_munmap,
        libc::SYS_brk,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_ioctl,
        libc::SYS_access,
        libc::SYS_dup,
        libc::SYS_dup2,
        libc::SYS_getpid,
        libc::SYS_getuid,
        libc::SYS_getgid,
        libc::SYS_geteuid,
        libc::SYS_getegid,
        libc::SYS_fcntl,
        libc::SYS_openat,
        libc::SYS_newfstatat,
        libc::SYS_exit,
        libc::SYS_exit_group,
        libc::SYS_futex,
        libc::SYS_getrandom,
        libc::SYS_clock_gettime,
        libc::SYS_clock_nanosleep,
    ]
}
