//! io_uring integration for zero-copy IPC

use io_uring::{opcode, types, IoUring};
use leeward_core::{LeewardError, Result};
use std::collections::HashMap;
use std::os::unix::io::RawFd;

/// Request ID for tracking io_uring operations
type RequestId = u64;

/// io_uring wrapper for batched async I/O
pub struct IoUringContext {
    /// The io_uring instance
    ring: IoUring,
    /// Next request ID
    next_id: RequestId,
    /// Active requests
    active_requests: HashMap<RequestId, ActiveRequest>,
}

/// An active request in the io_uring queue
struct ActiveRequest {
    /// Request ID
    id: RequestId,
    /// Operation type
    op_type: OpType,
    /// Buffer for data
    buffer: Vec<u8>,
}

/// Type of io_uring operation
#[derive(Debug, Clone, Copy)]
enum OpType {
    Read,
    Write,
}

impl IoUringContext {
    /// Create a new io_uring context
    pub fn new(queue_depth: u32) -> Result<Self> {
        let ring = IoUring::new(queue_depth)
            .map_err(|e| LeewardError::Execution(format!("failed to create io_uring: {e}")))?;

        Ok(Self {
            ring,
            next_id: 0,
            active_requests: HashMap::new(),
        })
    }

    /// Submit a read operation
    pub fn submit_read(&mut self, fd: RawFd, len: usize) -> Result<RequestId> {
        let id = self.next_id;
        self.next_id += 1;

        let mut buffer = vec![0u8; len];

        // Create read operation
        let read_op = opcode::Read::new(types::Fd(fd), buffer.as_mut_ptr(), len as u32)
            .build()
            .user_data(id);

        // SAFETY: Submitting io_uring operation
        unsafe {
            self.ring
                .submission()
                .push(&read_op)
                .map_err(|e| LeewardError::Execution(format!("failed to push read op: {e}")))?;
        }

        self.active_requests.insert(
            id,
            ActiveRequest {
                id,
                op_type: OpType::Read,
                buffer,
            },
        );

        Ok(id)
    }

    /// Submit a write operation
    pub fn submit_write(&mut self, fd: RawFd, data: Vec<u8>) -> Result<RequestId> {
        let id = self.next_id;
        self.next_id += 1;

        let len = data.len();

        // Create write operation
        let write_op = opcode::Write::new(types::Fd(fd), data.as_ptr(), len as u32)
            .build()
            .user_data(id);

        // SAFETY: Submitting io_uring operation
        unsafe {
            self.ring
                .submission()
                .push(&write_op)
                .map_err(|e| LeewardError::Execution(format!("failed to push write op: {e}")))?;
        }

        self.active_requests.insert(
            id,
            ActiveRequest {
                id,
                op_type: OpType::Write,
                buffer: data,
            },
        );

        Ok(id)
    }

    /// Submit all pending operations (single syscall)
    pub fn submit(&mut self) -> Result<usize> {
        self.ring
            .submit()
            .map_err(|e| LeewardError::Execution(format!("failed to submit io_uring: {e}")))
    }

    /// Wait for and process completions
    pub fn wait_completions(&mut self) -> Result<Vec<Completion>> {
        self.ring
            .submit_and_wait(1)
            .map_err(|e| LeewardError::Execution(format!("failed to wait for completions: {e}")))?;

        let mut completions = Vec::new();

        for cqe in self.ring.completion() {
            let id = cqe.user_data();
            let result = cqe.result();

            if let Some(req) = self.active_requests.remove(&id) {
                let completion = match req.op_type {
                    OpType::Read => {
                        if result < 0 {
                            Completion::Error {
                                id,
                                error: std::io::Error::from_raw_os_error(-result),
                            }
                        } else {
                            let data = req.buffer[..result as usize].to_vec();
                            Completion::Read { id, data }
                        }
                    }
                    OpType::Write => {
                        if result < 0 {
                            Completion::Error {
                                id,
                                error: std::io::Error::from_raw_os_error(-result),
                            }
                        } else {
                            Completion::Write {
                                id,
                                bytes_written: result as usize,
                            }
                        }
                    }
                };

                completions.push(completion);
            }
        }

        Ok(completions)
    }

    /// Check for completions without blocking
    pub fn try_completions(&mut self) -> Result<Vec<Completion>> {
        self.ring
            .submit()
            .map_err(|e| LeewardError::Execution(format!("failed to submit: {e}")))?;

        let mut completions = Vec::new();

        for cqe in self.ring.completion() {
            let id = cqe.user_data();
            let result = cqe.result();

            if let Some(req) = self.active_requests.remove(&id) {
                let completion = match req.op_type {
                    OpType::Read => {
                        if result < 0 {
                            Completion::Error {
                                id,
                                error: std::io::Error::from_raw_os_error(-result),
                            }
                        } else {
                            let data = req.buffer[..result as usize].to_vec();
                            Completion::Read { id, data }
                        }
                    }
                    OpType::Write => {
                        if result < 0 {
                            Completion::Error {
                                id,
                                error: std::io::Error::from_raw_os_error(-result),
                            }
                        } else {
                            Completion::Write {
                                id,
                                bytes_written: result as usize,
                            }
                        }
                    }
                };

                completions.push(completion);
            }
        }

        Ok(completions)
    }
}

/// Completion result from io_uring
#[derive(Debug)]
pub enum Completion {
    /// Read completed successfully
    Read { id: RequestId, data: Vec<u8> },
    /// Write completed successfully
    Write { id: RequestId, bytes_written: usize },
    /// Operation failed
    Error { id: RequestId, error: std::io::Error },
}
