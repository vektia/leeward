//! Shared memory for zero-copy result passing

use crate::{LeewardError, Result};
use memfd::{FileSeal, Memfd, MemfdOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicU32, Ordering};

/// Size of each request slot (64KB for code)
pub const REQUEST_SLOT_SIZE: usize = 64 * 1024;

/// Size of each response slot (1MB for stdout/stderr)
pub const RESPONSE_SLOT_SIZE: usize = 1024 * 1024;

/// Maximum number of concurrent requests
pub const MAX_SLOTS: usize = 64;

/// Shared memory region for request/response communication
pub struct SharedMemoryRegion {
    /// The memfd backing the shared memory
    memfd: Memfd,
    /// Number of active requests
    active_requests: AtomicU32,
    /// Request arena offset
    request_arena_offset: usize,
    /// Response arena offset
    response_arena_offset: usize,
}

impl SharedMemoryRegion {
    /// Create a new shared memory region
    pub fn new() -> Result<Self> {
        let opts = MemfdOptions::default()
            .allow_sealing(true)
            .close_on_exec(true);

        let memfd = opts
            .create("leeward_shm")
            .map_err(|e| LeewardError::Execution(format!("failed to create memfd: {e}")))?;

        // Calculate total size: request arena + response arena
        let request_arena_size = REQUEST_SLOT_SIZE * MAX_SLOTS;
        let response_arena_size = RESPONSE_SLOT_SIZE * MAX_SLOTS;
        let total_size = request_arena_size + response_arena_size;

        // Resize the memfd
        memfd
            .as_file()
            .set_len(total_size as u64)?;

        Ok(Self {
            memfd,
            active_requests: AtomicU32::new(0),
            request_arena_offset: 0,
            response_arena_offset: request_arena_size,
        })
    }

    /// Get the file descriptor for this shared memory
    pub fn as_raw_fd(&self) -> RawFd {
        self.memfd.as_raw_fd()
    }

    /// Allocate a request/response slot pair
    pub fn allocate_slot(&self) -> Result<SlotPair> {
        let slot_id = self.active_requests.fetch_add(1, Ordering::SeqCst);

        if slot_id >= MAX_SLOTS as u32 {
            self.active_requests.fetch_sub(1, Ordering::SeqCst);
            return Err(LeewardError::Execution(
                "no available slots in shared memory".into(),
            ));
        }

        Ok(SlotPair {
            slot_id,
            request_offset: self.request_arena_offset + (slot_id as usize * REQUEST_SLOT_SIZE),
            response_offset: self.response_arena_offset + (slot_id as usize * RESPONSE_SLOT_SIZE),
            memfd_fd: self.as_raw_fd(),
        })
    }

    /// Free a slot
    pub fn free_slot(&self, _slot: SlotPair) {
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
    }

    /// Seal the memfd to prevent further modifications
    pub fn seal(&mut self) -> Result<()> {
        self.memfd
            .add_seals(&[FileSeal::SealShrink, FileSeal::SealGrow])
            .map_err(|e| LeewardError::Execution(format!("failed to seal memfd: {e}")))?;
        Ok(())
    }
}

/// A pair of request/response slots in shared memory
#[derive(Debug, Clone)]
pub struct SlotPair {
    /// Unique slot ID
    pub slot_id: u32,
    /// Offset into shared memory for request data
    pub request_offset: usize,
    /// Offset into shared memory for response data
    pub response_offset: usize,
    /// File descriptor for the memfd
    pub memfd_fd: RawFd,
}

impl SlotPair {
    /// Get a pointer to the request data
    ///
    /// # Safety
    /// The caller must ensure the shared memory is properly mapped
    pub unsafe fn request_ptr(&self) -> *mut u8 {
        // This would be used after mmap() to get actual pointer
        // For now, just return the offset as a pointer (will be fixed in actual mmap usage)
        self.request_offset as *mut u8
    }

    /// Get a pointer to the response data
    ///
    /// # Safety
    /// The caller must ensure the shared memory is properly mapped
    pub unsafe fn response_ptr(&self) -> *mut u8 {
        self.response_offset as *mut u8
    }
}

/// Memory-mapped view of the shared memory region
pub struct MappedSharedMemory {
    /// Base pointer to mapped memory
    base_ptr: *mut libc::c_void,
    /// Total size of the mapping
    size: usize,
}

impl MappedSharedMemory {
    /// Map a shared memory file descriptor into this process
    pub fn new(fd: RawFd, read_only: bool) -> Result<Self> {
        let request_arena_size = REQUEST_SLOT_SIZE * MAX_SLOTS;
        let response_arena_size = RESPONSE_SLOT_SIZE * MAX_SLOTS;
        let total_size = request_arena_size + response_arena_size;

        let prot = if read_only {
            libc::PROT_READ
        } else {
            libc::PROT_READ | libc::PROT_WRITE
        };

        // SAFETY: mmap syscall to map shared memory
        let base_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total_size,
                prot,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if base_ptr == libc::MAP_FAILED {
            return Err(LeewardError::Io(std::io::Error::last_os_error()));
        }

        Ok(Self {
            base_ptr,
            size: total_size,
        })
    }

    /// Write code to a request slot
    pub fn write_request(&self, slot: &SlotPair, code: &[u8]) -> Result<()> {
        if code.len() > REQUEST_SLOT_SIZE {
            return Err(LeewardError::Execution(format!(
                "code too large: {} bytes (max {})",
                code.len(),
                REQUEST_SLOT_SIZE
            )));
        }

        // SAFETY: Writing to mapped shared memory
        unsafe {
            let dest = (self.base_ptr as *mut u8).add(slot.request_offset);
            std::ptr::copy_nonoverlapping(code.as_ptr(), dest, code.len());
            // Write length prefix
            let len_ptr = dest.cast::<u32>();
            *len_ptr = code.len() as u32;
        }

        Ok(())
    }

    /// Read response from a response slot
    pub fn read_response(&self, slot: &SlotPair) -> Result<Vec<u8>> {
        // SAFETY: Reading from mapped shared memory
        unsafe {
            let src = (self.base_ptr as *const u8).add(slot.response_offset);
            // Read length prefix
            let len = *(src.cast::<u32>());

            if len as usize > RESPONSE_SLOT_SIZE {
                return Err(LeewardError::Execution(format!(
                    "response too large: {} bytes",
                    len
                )));
            }

            let mut buffer = vec![0u8; len as usize];
            std::ptr::copy_nonoverlapping(src.add(4), buffer.as_mut_ptr(), len as usize);
            Ok(buffer)
        }
    }
}

impl Drop for MappedSharedMemory {
    fn drop(&mut self) {
        // SAFETY: Unmapping the shared memory
        unsafe {
            libc::munmap(self.base_ptr, self.size);
        }
    }
}

// SAFETY: Shared memory can be safely sent between threads
unsafe impl Send for MappedSharedMemory {}
unsafe impl Sync for MappedSharedMemory {}
