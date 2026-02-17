# Xpresso (work in progress)

Xpresso is a high-performance kernel-bypass networking library built with Rust. It provides a simple `Context → Program → send/recv` API, with goal of supporting XDP/AF_XDP on Linux and Skywalk(or BPF?) on macOS.

Currently uses a UDP socket fallback backend, so examples run on any OS without root.

## Prerequisites

### General
- Rust (Nightly toolchain required for eBPF compilation)
- `cargo-bpf` or `bpf-linker` (installed via `cargo install bpf-linker`)

### Linux
- Kernel 5.10+ (for full XDP support)
- Build tools: `clang`, `llvm`, `libelf-dev`, `pkg-config`

### macOS
- **OrbStack** (v0.4.0+) is **REQUIRED** for local XDP development.
  - *Note: Docker Desktop for Mac is NOT supported due to kernel limitations (`AF_XDP` protocol error).*

## Quick Start

### 1. Build & Run Local (UDP Mode)
For quick logic testing on macOS/Linux without root/XDP:

```bash
# Server
cargo run -p xpresso-echo -- server --port 9000

# Client
cargo run -p xpresso-echo -- client --server 127.0.0.1:9000 --message "hello xpresso"
```

### 2. Run XDP Mode (Docker/OrbStack)
To test actual XDP packet processing (Linux/OrbStack):

```bash
# Enter the test directory
cd sample-test

# Start container (requires privileged mode for eBPF)
docker compose up --build -d

# Check logs to confirm XDP attachment
docker compose logs -f xpresso
# Output should contain: "[xpresso] AF_XDP: iface=eth0 ... attached on ..."
```

## API Usage

```rust
use xpresso::{Context, Mode, Program};

// 1. Configure
let mut ctx = Context::new(Mode::Echo);
ctx.set_port(9000);

// 2. Attach
let mut prog = Program::with_context(ctx)?;
prog.attach()?;

// 3. Send / Receive
prog.send(&dest_addr, b"data")?;
let (from, n) = prog.recv(&mut buf)?;

// 4. Stats
println!("{}", prog.stats());
```

## Project Structure

```
xpresso/
├── src/              # Core library (Context, Program, Stats)
├── xpresso-ebpf/     # XDP eBPF program (Linux, compiled separately)
├── sample-test/
    └── xpresso-echo/ # Echo server/client example
```
