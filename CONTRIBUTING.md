# Contributing to Leeward

Thank you for your interest in contributing to Leeward! This document provides guidelines and information for contributors.

## 🚀 Quick Start

### Development Setup

#### With Nix (Recommended)
```bash
# Clone the repository
git clone https://github.com/vektia/leeward
cd leeward

# Enter development shell (auto-loads all dependencies)
nix develop

# Or with direnv (automatic)
direnv allow
```

#### Without Nix
```bash
# Install Rust 1.85+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/vektia/leeward
cd leeward
cargo build --release
```

### Running Locally

```bash
# Start the daemon
./target/release/leeward-daemon &

# Or with cargo alias
cargo daemon &

# Execute code
./target/release/leeward exec "print('Hello, World!')"

# Or with cargo alias
cargo cli exec "print('Hello, World!')"
```

## 🏗️ Architecture Overview

Leeward uses a pre-fork pool architecture for ultra-low latency (~0.5ms):

```
Client (any language) ←→ Unix Socket ←→ Daemon ←→ Pre-warmed Workers
                                                    ├─ Python ready
                                                    ├─ Isolated
                                                    └─ Waiting
```

### Key Components

- **leeward-core**: Core isolation primitives (namespaces, seccomp, landlock)
- **leeward-daemon**: Persistent daemon managing worker pool
- **leeward-cli**: Command-line interface
- **leeward-ffi**: C FFI for language bindings

### Isolation Layers

1. **Linux Namespaces** - Process, network, mount isolation
2. **Seccomp** - Syscall filtering
3. **Landlock** - Filesystem access control

## 📝 Code Style

### Rust Guidelines

- Use Rust 2024 edition
- Enable clippy lints: `clippy::all`, `clippy::pedantic`, `clippy::nursery`
- Use `thiserror` for error types
- Use `tracing` for logging (not `println!`)
- Document public APIs with examples

### Running Checks

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features

# Run tests
cargo test --workspace

# Check everything
cargo check && cargo clippy && cargo test
```

## 🧪 Testing

### Unit Tests
Place unit tests in the same file as the code:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

### Integration Tests
Place integration tests in `tests/integration/`:
```rust
// tests/integration/basic_execution.rs
#[test]
fn test_python_execution() {
    // ...
}
```

### Security Tests
Security/escape tests go in `tests/escapes/`:
```rust
// tests/escapes/network_escape.rs
#[test]
fn test_network_isolation() {
    // Should fail - no network in sandbox
}
```

## 🔧 Making Changes

### 1. Fork and Branch
```bash
git checkout -b feature/your-feature
# or
git checkout -b fix/your-bugfix
```

### 2. Make Your Changes
- Write clean, documented code
- Add tests for new functionality
- Update documentation if needed

### 3. Commit Guidelines
Follow conventional commits:
```
feat: add new feature
fix: fix bug in worker pool
docs: update README
test: add security tests
refactor: simplify isolation code
perf: optimize worker spawning
```

### 4. Submit PR
- Fill out the PR template
- Ensure CI passes
- Wait for review

## 🚢 Release Process

### Building Release Artifacts

#### Local Build with Nix
```bash
# Build all components
nix build .#daemon
nix build .#cli

# Build Debian package
nix build -f nix/deb.nix

# Create static binary
cargo build --release --target x86_64-unknown-linux-musl
```

#### GitHub Actions
Releases are automatically built when pushing tags:
```bash
git tag v0.1.0
git push origin v0.1.0
```

This creates:
- Linux binaries (x86_64, aarch64)
- Debian packages (.deb)
- Static musl binaries
- Nix bundles
- SHA256 checksums

### Using the Nix Overlay

```nix
# In your flake.nix
{
  inputs.leeward.url = "github:vektia/leeward";

  outputs = { self, nixpkgs, leeward, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        {
          nixpkgs.overlays = [ leeward.overlays.default ];
          environment.systemPackages = [ pkgs.leeward ];
        }
      ];
    };
  };
}
```

## 📋 TODO for v1.0

High priority items for the v1.0 release:

- [ ] Implement io_uring IPC for zero-copy communication
- [ ] Add shared memory support (memfd + mmap)
- [ ] Implement SECCOMP_USER_NOTIF for syscall supervision
- [ ] Complete Python bindings
- [ ] Add timeout handling in workers
- [ ] Implement worker recycling after N executions
- [ ] Add metrics/monitoring
- [ ] Complete FFI for other languages

## 🐛 Debugging

### Enable Debug Logging
```bash
RUST_LOG=debug ./target/release/leeward-daemon
```

### Check Worker Status
```bash
./target/release/leeward status
```

### Common Issues

**"Operation not permitted" on namespaces**
```bash
# Check if user namespaces are enabled
cat /proc/sys/kernel/unprivileged_userns_clone
# Should be 1
```

**Landlock not available**
```bash
# Need Linux >= 5.13
uname -r
```

## 📚 Resources

- [Linux Namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html)
- [Seccomp](https://man7.org/linux/man-pages/man2/seccomp.2.html)
- [Landlock](https://docs.kernel.org/userspace-api/landlock.html)
- [io_uring](https://kernel.dk/io_uring.pdf)

## 📄 License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.

## 🤝 Code of Conduct

Be respectful, inclusive, and constructive. We're building secure software that protects users - let's do it together professionally.

## 💬 Questions?

- Open an issue for bugs/features
- Start a discussion for questions
- Email: hello@vektia.com.br

Happy coding! 🚀