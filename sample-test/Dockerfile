# === Builder Stage ===
FROM rust:1.87-bookworm as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    clang \
    llvm \
    libelf-dev \
    pkg-config \
    iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Install nightly toolchain and bpf-linker
RUN rustup toolchain install nightly --component rust-src \
    && rustup target add --toolchain nightly x86_64-unknown-linux-gnu \
    && cargo +nightly install bpf-linker

WORKDIR /flight

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY flight-ebpf/Cargo.toml flight-ebpf/Cargo.toml
COPY sample-test/flight-echo/Cargo.toml sample-test/flight-echo/Cargo.toml

# Create dummy sources to cache dependencies
RUN mkdir -p src && echo "" > src/lib.rs \
    && mkdir -p src/platform && echo "" > src/platform/mod.rs \
    && mkdir -p flight-ebpf/src && echo '#![no_std] #![no_main] #[panic_handler] fn p(_:&core::panic::PanicInfo)->!{loop{}}' > flight-ebpf/src/main.rs \
    && mkdir -p sample-test/flight-echo/src && echo "fn main(){}" > sample-test/flight-echo/src/main.rs

# Build dependencies only (both for userspace and eBPF if possible, though eBPF usually needs special flags)
# We focus on userspace deps here to save time
RUN cargo build --workspace --release || true

# Clean up dummy sources
RUN rm -rf src flight-ebpf/src sample-test/flight-echo/src

# Copy actual source code
COPY . .

# Ensure source files are newer than cached artifacts (fix for mtime issue)
RUN find . -name "*.rs" -exec touch {} +

# Build XDP program (flight-ebpf)
RUN cd flight-ebpf && cargo +nightly build \
    --target bpfel-unknown-none \
    -Z build-std=core \
    --release

# Build echo tool (flight-echo)
RUN cargo build --package flight-echo --release

# === Runtime Stage ===
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    iproute2 \
    iputils-ping \
    net-tools \
    ca-certificates \
    ethtool \
    bpftool \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
# flight-echo loads the eBPF program embedded in itself
COPY --from=builder /flight/target/release/flight-echo /app/flight-echo
COPY --from=builder /flight/flight-ebpf/target/bpfel-unknown-none/release/flight-redirect /app/flight-redirect

# Set capabilities (Docker Compose handles this, but good to note)
# Default CMD run the echo server
CMD ["/app/flight-echo", "server", "--port", "9000", "--iface", "eth0"]
