# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**leeward** is a Linux-native sandbox for running untrusted code (currently Python) with extremely low latency (~0.5ms vs Docker's 300-500ms). It's designed for AI agent code execution using native Linux primitives instead of containers or VMs.

**Status:** Work in progress. Implementing advanced pre-fork pool with io_uring and shared memory.

## Architecture

### Pre-Fork Execution Model
```
┌──────────────┐   io_uring/shm    ┌─────────────────────────────────┐
│    Client    │ ◄───────────────► │      leeward daemon             │
│   (any lang) │   zero-copy IPC   │                                 │
└──────────────┘                   │  ┌───────────────────────────┐  │
                                   │  │ Pre-warmed Worker Pool    │  │
      Python, Go,                  │  │                           │  │
      Node, Rust                   │  │ [W1] Python idle ──pipe   │  │
      via C FFI                    │  │ [W2] Python idle ──pipe   │  │
                                   │  │ [W3] Python idle ──pipe   │  │
                                   │  └───────────────────────────┘  │
                                   └─────────────────────────────────┘
```

- **leeward-daemon**: Persistent daemon maintaining pre-forked worker pool
- **leeward-cli**: Command-line interface for executing code
- **leeward-ffi**: C FFI library for multi-language support
- **leeward-core**: Core isolation primitives and worker management

### Performance Architecture (4 Levels)

**Level 1: Pre-fork Pool (~0.5ms)**
- Workers created at daemon startup using `clone3` with `CLONE_INTO_CGROUP`
- Python interpreter already loaded and idle
- Code sent via pipe, execution happens immediately
- No fork/exec overhead on each request

**Level 2: io_uring IPC (~0.2ms savings)**
- Zero-copy async I/O using io_uring submission queues
- 1 syscall per batch vs 4 syscalls per request (write/read/write/read)
- Batched request processing

**Level 3: Shared Memory (~0.1ms savings)**
- Results written to `memfd_create` + `mmap` shared region
- Client and daemon map same file descriptor
- Eliminates 2 kernel copies (worker→daemon→client becomes worker→shared memory)
- Request/response arenas in shared memory

**Level 4: SECCOMP_USER_NOTIF (no worker recycling)**
- Blocked syscalls notify supervisor instead of killing process
- Supervisor returns `EACCES` to continue execution
- Workers don't die on denied syscalls, no recycling needed
- Graceful degradation vs fatal termination

### Isolation Layers (Defense in Depth)
1. **Linux namespaces** via `clone3` (user, pid, mount, net, ipc, uts)
2. **seccomp user notifications** (`SECCOMP_USER_NOTIF`) - supervisor-mediated syscall filtering
3. **Landlock** filesystem restrictions (whitelist-based)
4. **cgroups v2** resource limits via `CLONE_INTO_CGROUP` (256MB RAM, 100% CPU, 32 PIDs, 30s timeout)

### Communication Protocol
- **Client ↔ Daemon**: io_uring submission queue or Unix socket fallback
- **Daemon ↔ Worker**: Anonymous pipes for code delivery
- **Results**: Shared memory region (memfd) mapped by both daemon and client
- **Serialization**: MessagePack for control messages
- **Worker lifecycle**: Long-lived, survives denied syscalls (SECCOMP_USER_NOTIF)

## Workspace Structure

```
crates/
├── leeward-core/          # Core isolation primitives
│   └── src/
│       ├── config.rs      # SandboxConfig with resource limits
│       ├── worker.rs      # Worker process management
│       ├── result.rs      # ExecutionResult, ExecutionMetrics
│       ├── error.rs       # Error types
│       └── isolation/     # Isolation mechanisms
│           ├── namespace.rs   # setup_namespaces()
│           ├── seccomp.rs     # setup_seccomp()
│           ├── landlock.rs    # setup_landlock()
│           ├── cgroups.rs     # setup_cgroups()
│           └── mounts.rs      # setup_mounts()
├── leeward-daemon/        # Persistent daemon
│   └── src/
│       ├── main.rs        # Entry point, signal handling
│       ├── config.rs      # DaemonConfig (pool size, socket path)
│       ├── pool.rs        # WorkerPool management
│       ├── server.rs      # UnixServer for client connections
│       └── protocol.rs    # Request/Response types
├── leeward-cli/           # CLI interface
│   └── src/
│       └── main.rs        # Commands: exec, status, ping, run
└── leeward-ffi/           # C FFI bindings
    └── src/
        └── lib.rs         # leeward_execute(), leeward_free_result()
```

## Build System

### Cargo (Primary)
```bash
# Development
cargo build
cargo test
cargo check
cargo clippy

# Aliases (from .cargo/config.toml)
cargo b          # build
cargo br         # build --release
cargo t          # test
cargo c          # check
cargo cl         # clippy
cargo daemon     # run daemon in release mode
cargo cli        # run CLI in release mode

# Release build (with LTO, single codegen unit, stripped)
cargo build --release

# Test specific crate
cargo test -p leeward-core
cargo test --workspace
```

### Nix (Optional, for reproducible builds)
```bash
# Build specific targets
nix build .#cli      # CLI binary
nix build .#daemon   # Daemon binary
nix build .#ffi      # C FFI library (.so and .a)
nix build            # Default: all targets

# Development shell (includes Rust, mold linker, all deps)
nix develop
```

### Build Configuration
- **Rust version:** 1.85 (edition 2024)
- **Linker:** mold (via clang) for faster builds
- **Release optimizations:** LTO=fat, codegen-units=1, strip=true, opt-level=3
- **Dependencies optimized** at opt-level=3 even in dev builds (for package.*)

## Development Commands

### Running the daemon
```bash
cargo daemon
# Or with args:
cargo run --release --bin leeward-daemon -- --pool-size 10
```

### Using the CLI
```bash
cargo cli exec "print('hello world')"
cargo cli status
cargo cli ping
```

### Testing
```bash
cargo test                    # All tests
cargo test --workspace        # Explicit workspace tests
cargo test -p leeward-core    # Single crate
```

Test locations:
- `tests/integration/` - Integration tests
- `tests/escapes/` - Security/escape tests
- Unit tests within each crate's `src/` files

## Key Technical Details

### Requirements
- Linux >= 5.13 (Landlock support required)
- User namespaces enabled (check: `unshare -U whoami`)
- No root required

### Default Resource Limits
```rust
// See crates/leeward-core/src/config.rs
memory_limit: 256MB
cpu_quota: 100%
max_processes: 32
timeout: 30s
```

### Seccomp Syscall Whitelist
Only essential Python syscalls allowed. See `crates/leeward-core/src/isolation/seccomp.rs` for the full list (currently marked TODO).

### Multi-Language Support
The C FFI library (`leeward-ffi`) enables bindings for any language with C interop:
- Python (planned in `bindings/python/`)
- Go, Node.js, Rust - all can use the FFI
- Generated header file via cbindgen
- Both shared (`.so`) and static (`.a`) libraries built

## Important Implementation Notes

### Currently Marked TODO
Implementing new execution paradigm:
- `crates/leeward-core/src/isolation/*.rs` - All setup functions
- Pre-fork pool with `clone3` + `CLONE_INTO_CGROUP`
- Pipe-based code delivery to idle workers
- io_uring integration for IPC
- Shared memory (memfd + mmap) for results
- SECCOMP_USER_NOTIF for syscall supervision

When implementing, maintain the defense-in-depth approach: all 4 isolation layers must work together.

### Performance Implementation Details

**Pre-fork Pool:**
- Use `clone3` syscall with `CLONE_INTO_CGROUP` flag
- Workers created at daemon startup, not on-demand
- Each worker loads Python interpreter and enters idle loop
- Communication via anonymous pipes (one per worker)
- Workers never exit unless recycled (every N executions or on error)

**io_uring Integration:**
- Daemon maintains io_uring instance for client communication
- Submission queue batches multiple requests
- Zero-copy buffers using registered buffers
- Completion queue processed asynchronously
- Fallback to Unix socket if io_uring unavailable

**Shared Memory:**
- Create with `memfd_create("leeward_results", MFD_CLOEXEC)`
- Request arena: fixed-size slots for incoming code
- Response arena: variable-size slots for stdout/stderr
- Client maps read-only, daemon/workers map read-write
- Ring buffer or slab allocator for slot management

**SECCOMP_USER_NOTIF:**
- Set up seccomp filter with `SECCOMP_RET_USER_NOTIF` for blocked syscalls
- Supervisor thread polls seccomp notification fd
- On notification: log syscall, return `EACCES` to continue
- Worker continues execution instead of dying
- Reduces worker churn dramatically

### Security Considerations
- No network access by default (isolated network namespace)
- Filesystem access via Landlock whitelist only
- Resource limits enforced via cgroups v2
- Syscalls mediated by supervisor (SECCOMP_USER_NOTIF)
- Workers are long-lived but fully isolated
- Shared memory regions are per-request, isolated between executions

### Code Style
- Workspace uses Rust 2024 edition
- Lints: clippy::all, clippy::pedantic, clippy::nursery enabled
- Unsafe code generates warnings (required for isolation primitives)
- Use `thiserror` for error types
- Use `tracing` for logging, not `println!`

## License and Support

- **License:** MIT (previously Apache-2.0, changed in recent commits)
- **Repository:** https://github.com/vektia/leeward
- **Production usage:** See ADOPTION.md to report usage
- **Enterprise support:** hello@vektia.com.br
- **Sponsors:** github.com/sponsors/vektia
