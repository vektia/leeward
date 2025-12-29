# leeward

> Linux-native sandbox for running untrusted code. No containers. No VMs. Fast.

⚠️ **Work in progress** — Core isolation primitives are being implemented.

## Why

AI agents need to execute code. Current options suck:

| Solution | Problem |
|----------|---------|
| Docker | 300-500ms startup, heavy |
| E2B/Modal | Cloud-only, expensive |
| WASM | No native libs, limited |
| Firecracker | Overkill for most cases |

leeward gives you **~0.5ms** execution latency using native Linux primitives.

## How
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

Each worker is isolated with:
- Linux namespaces (user, pid, mount, net, ipc) via clone3
- seccomp user notifications (supervisor decides on blocked syscalls)
- Landlock filesystem restrictions
- cgroups v2 resource limits (CLONE_INTO_CGROUP)

## Usage
```python
from leeward import Leeward

with Leeward() as sandbox:
    result = sandbox.execute("print(sum(range(100)))")
    print(result.stdout)  # "4950"
```
```bash
# Or via CLI
leeward exec "print('hello')"
```

## Requirements

- Linux >= 5.13 (Landlock support)
- User namespaces enabled
- No root required

## Status

Building the core. Not ready for production.

## Support

leeward is free and open source under [Apache-2.0](LICENSE.md).

- **Using in production?** [Let us know](ADOPTION.md) — it helps the project
- **Sponsors** — [github.com/sponsors/vektia](https://github.com/sponsors/vektia)
- **Enterprise support** — hello@vektia.com.br

## License

[Apache-2.0](LICENSE.md)