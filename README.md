# Xpresso (work in progress)

Xpresso is a Linux-primary Rust crate that takes the kernel out of the UDP datapath. It is an **AF_XDP-backed unreliable datagram socket** with an in-kernel **XDP filter/policy control plane** (runtime-mutable drop/pass/redirect rules). The goal is narrow: fewer syscalls and fewer copies than a standard `recvfrom`/`sendto` UDP socket, while the bytes on the wire stay ordinary UDP.

It is not a transport protocol. The core provides no reliability, ordering, congestion control, or encryption, and it does not replace QUIC — if you need a reliable encrypted stream, use `quinn`. For the full positioning and a packet-by-packet comparison of the standard UDP datapath vs the AF_XDP datapath, see [`docs/DESIGN.md`](docs/DESIGN.md) and [`docs/BLOG_UDP_DATAPATH_VS_AFXDP.md`](docs/BLOG_UDP_DATAPATH_VS_AFXDP.md).

**Platforms:** kernel bypass is **Linux only**. macOS uses a plain `UdpSocket` fallback (same wire format, **no kernel bypass**) so examples build and run anywhere without root — it is a development convenience, not a performance backend.

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
