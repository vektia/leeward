//! Linux isolation primitives
//!
//! This module contains the core isolation mechanisms:
//! - `clone3` - clone3 syscall for process creation
//! - `namespace` - Linux namespaces (user, pid, mount, net, ipc)
//! - `seccomp` - syscall filtering with SECCOMP_USER_NOTIF
//! - `landlock` - filesystem access control
//! - `mounts` - filesystem setup with bind mounts and tmpfs

pub mod clone3;
pub mod landlock;
pub mod mounts;
pub mod namespace;
pub mod seccomp;

pub use self::landlock::LandlockConfig;
pub use self::mounts::MountConfig;
pub use self::namespace::NamespaceConfig;
pub use self::seccomp::SeccompConfig;
