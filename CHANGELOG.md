# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of leeward
- Pre-fork worker pool architecture for ~0.5ms latency
- Linux namespace isolation (pid, mount, net, ipc, uts)
- Seccomp syscall filtering
- Landlock filesystem restrictions
- Unix socket IPC between daemon and CLI
- MessagePack protocol for communication
- Systemd service files (system and user)
- Nix flake support with overlay
- Debian package generation
- Static musl binary builds
- Multi-architecture support (x86_64, aarch64)

### Architecture
- `leeward-core`: Core isolation primitives
- `leeward-daemon`: Persistent daemon with worker pool
- `leeward-cli`: Command-line interface
- `leeward-ffi`: C FFI library for language bindings

### Security Features
- No root required (uses user namespaces)
- Defense in depth with 3 isolation layers
- Zero network access by default
- Restricted filesystem access via Landlock
- Minimal syscall whitelist via seccomp

## [0.1.0] - TBD

Initial public release.