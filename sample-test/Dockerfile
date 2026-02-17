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

WORKDIR /xpresso

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY xpresso-ebpf/Cargo.toml xpresso-ebpf/Cargo.toml
COPY sample-test/xpresso-echo/Cargo.toml sample-test/xpresso-echo/Cargo.toml

# Create dummy sources to cache dependencies
RUN mkdir -p src && echo "" > src/lib.rs \
    && mkdir -p src/platform && echo "" > src/platform/mod.rs \
    && mkdir -p xpresso-ebpf/src && echo '#![no_std] #![no_main] #[panic_handler] fn p(_:&core::panic::PanicInfo)->!{loop{}}' > xpresso-ebpf/src/main.rs \
    && mkdir -p sample-test/xpresso-echo/src && echo "fn main(){}" > sample-test/xpresso-echo/src/main.rs

# Build dependencies only (both for userspace and eBPF if possible, though eBPF usually needs special flags)
# We focus on userspace deps here to save time
RUN cargo build --workspace --release || true

# Clean up dummy sources
RUN rm -rf src xpresso-ebpf/src sample-test/xpresso-echo/src

# Copy actual source code
COPY . .

# Ensure source files are newer than cached artifacts (fix for mtime issue)
RUN find . -name "*.rs" -exec touch {} +

# Build XDP program (xpresso-ebpf)
RUN cd xpresso-ebpf && cargo +nightly build \
    --target bpfel-unknown-none \
    -Z build-std=core \
    --release

# Build echo tool (xpresso-echo)
RUN cargo build --package xpresso-echo --release

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
# xpresso-echo loads the eBPF program embedded in itself
COPY --from=builder /xpresso/target/release/xpresso-echo /app/xpresso-echo
COPY --from=builder /xpresso/xpresso-ebpf/target/bpfel-unknown-none/release/xpresso-redirect /app/xpresso-redirect

# Set capabilities (Docker Compose handles this, but good to note)
# Default CMD run the echo server
CMD ["/app/xpresso-echo", "server", "--port", "9000", "--iface", "eth0"]
