# Installation Guide

## Quick Install

### From Release (Binary)

Download the latest release from [GitHub Releases](https://github.com/vektia/leeward/releases):

```bash
# Linux x86_64
curl -L https://github.com/vektia/leeward/releases/latest/download/leeward-amd64-linux.tar.gz | tar xz
sudo mv leeward/leeward* /usr/local/bin/

# Linux ARM64
curl -L https://github.com/vektia/leeward/releases/latest/download/leeward-arm64-linux.tar.gz | tar xz
sudo mv leeward/leeward* /usr/local/bin/
```

### Debian/Ubuntu

```bash
# Download and install the .deb package
wget https://github.com/vektia/leeward/releases/latest/download/leeward_*_amd64.deb
sudo dpkg -i leeward_*.deb

# The daemon will be installed as a systemd service
sudo systemctl start leeward
sudo systemctl enable leeward
```

### Nix (Flakes)

Add to your `flake.nix`:

```nix
{
  inputs = {
    leeward.url = "github:vektia/leeward";
  };

  outputs = { self, nixpkgs, leeward, ... }: {
    # Use as overlay
    nixpkgs.overlays = [ leeward.overlays.default ];

    # Or add to system packages
    environment.systemPackages = [ leeward.packages.${system}.default ];
  };
}
```

Direct installation:

```bash
# Install CLI and daemon
nix profile install github:vektia/leeward#cli
nix profile install github:vektia/leeward#daemon

# Or run directly
nix run github:vektia/leeward#daemon
nix run github:vektia/leeward#cli -- exec "print('hello')"
```

### From Source

```bash
# Clone repository
git clone https://github.com/vektia/leeward
cd leeward

# Build with Cargo
cargo build --release

# Install binaries
sudo cp target/release/leeward-daemon /usr/local/bin/
sudo cp target/release/leeward /usr/local/bin/

# Or build with Nix
nix build .#daemon
nix build .#cli
```

## Platform-Specific Instructions

### NixOS

Add to your `configuration.nix`:

```nix
{ config, pkgs, ... }:
{
  # Import the module
  imports = [
    "${builtins.fetchTarball "https://github.com/vektia/leeward/archive/main.tar.gz"}/nix/module.nix"
  ];

  # Enable the service
  services.leeward = {
    enable = true;
    workers = 4;  # Number of pre-forked workers
  };

  # Enable user namespaces
  boot.kernel.sysctl."kernel.unprivileged_userns_clone" = 1;
}
```

### Ubuntu/Debian

```bash
# Enable user namespaces (if not already enabled)
echo 'kernel.unprivileged_userns_clone=1' | sudo tee /etc/sysctl.d/99-userns.conf
sudo sysctl --system

# Install from .deb
wget https://github.com/vektia/leeward/releases/latest/download/leeward_*_amd64.deb
sudo dpkg -i leeward_*.deb

# Start service
sudo systemctl enable --now leeward
```

### Fedora/RHEL/Rocky

```bash
# Enable user namespaces (if disabled)
sudo sysctl -w kernel.unprivileged_userns_clone=1
echo 'kernel.unprivileged_userns_clone=1' | sudo tee /etc/sysctl.d/99-userns.conf

# Install from tarball
curl -L https://github.com/vektia/leeward/releases/latest/download/leeward-amd64-linux.tar.gz | tar xz
sudo mv leeward/* /usr/local/bin/

# Create systemd service
sudo cp leeward/leeward.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now leeward
```

### Arch Linux

```bash
# User namespaces are enabled by default

# Install from AUR (when available)
yay -S leeward

# Or from tarball
curl -L https://github.com/vektia/leeward/releases/latest/download/leeward-amd64-linux.tar.gz | tar xz
sudo mv leeward/* /usr/local/bin/
```

### Alpine Linux (Static Binary)

```bash
# Download static musl binary
wget https://github.com/vektia/leeward/releases/latest/download/leeward-static-linux-amd64.tar.gz
tar xzf leeward-static-linux-amd64.tar.gz
sudo mv leeward-static/* /usr/local/bin/
```

## Running as a Service

### Systemd (System Service)

```bash
# Copy the service file
sudo cp contrib/leeward.system.service /etc/systemd/system/leeward.service

# Create leeward user
sudo useradd -r -s /usr/sbin/nologin -d /nonexistent leeward

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now leeward

# Check status
sudo systemctl status leeward
journalctl -u leeward -f
```

### Systemd (User Service)

```bash
# Copy user service file
mkdir -p ~/.config/systemd/user/
cp contrib/leeward.user.service ~/.config/systemd/user/leeward.service

# Start user service
systemctl --user daemon-reload
systemctl --user enable --now leeward

# Check status
systemctl --user status leeward
```

### Docker (Alternative)

If you prefer containerized deployment:

```bash
# Using the official image (when available)
docker run -d \
  --privileged \
  --name leeward \
  -v /run/leeward:/run/leeward \
  ghcr.io/vektia/leeward:latest

# Or build from source
docker build -t leeward .
docker run -d --privileged --name leeward leeward
```

## Verification

After installation, verify everything works:

```bash
# Check daemon is running
leeward status

# Execute test code
leeward exec "print('Hello from Leeward!')"

# Test isolation (should fail)
leeward exec "import socket; socket.socket()"  # No network
leeward exec "open('/etc/passwd', 'r')"        # No filesystem access
```

## Building Packages

### Build Debian Package

With Nix:
```bash
nix build -f nix/deb.nix
# Output: result/leeward_0.1.0_amd64.deb
```

### Build Static Binary

```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl
```

### Build All Release Artifacts

```bash
# With Nix (recommended)
nix build .#daemon
nix build .#cli
nix build -f nix/deb.nix

# With Cargo
cargo build --release
cargo build --release --target x86_64-unknown-linux-musl
```

## Troubleshooting

### "Operation not permitted" on namespaces

Check if user namespaces are enabled:
```bash
cat /proc/sys/kernel/unprivileged_userns_clone
# Should output: 1

# If not, enable:
sudo sysctl -w kernel.unprivileged_userns_clone=1
```

### Landlock not available

Requires Linux kernel >= 5.13:
```bash
uname -r  # Check kernel version

# Check if Landlock is enabled
cat /sys/kernel/security/lsm | grep landlock
```

### Socket permission denied

Ensure the socket directory exists and has correct permissions:
```bash
# For system service
sudo mkdir -p /run/leeward
sudo chown leeward:leeward /run/leeward

# For user service
mkdir -p $XDG_RUNTIME_DIR/leeward
```

## Uninstall

### From Package Manager
```bash
# Debian/Ubuntu
sudo apt remove leeward

# With systemctl
sudo systemctl stop leeward
sudo systemctl disable leeward
```

### Manual Uninstall
```bash
# Stop service
sudo systemctl stop leeward

# Remove binaries
sudo rm /usr/local/bin/leeward*

# Remove service files
sudo rm /etc/systemd/system/leeward.service

# Remove user (if created)
sudo userdel leeward

# Remove runtime directory
sudo rm -rf /run/leeward
```