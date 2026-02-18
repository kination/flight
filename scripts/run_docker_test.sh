#!/bin/bash
set -e

# Change to the project root directory (assuming script is in scripts/)
cd "$(dirname "$0")/.."

IMAGE_NAME="xpresso-test-linux"

echo "Building Docker image: $IMAGE_NAME"
docker build --build-arg CACHE_BUST="$(date +%s)" -f scripts/Dockerfile.test -t $IMAGE_NAME .

echo "Running tests in Docker container..."
# Note: --privileged is required for loading eBPF programs and creating AF_XDP sockets.
docker run --rm --privileged \
    $IMAGE_NAME \
    /bin/bash -c "cd xpresso-ebpf && cargo +nightly build --release --target=bpfel-unknown-none -Z build-std=core && cd .. && cargo test --workspace --lib -- --test-threads=1 && cargo test --workspace --test context_tests -- --test-threads=1 && cargo test --workspace --test error_tests -- --test-threads=1 && cargo test --workspace --test program_tests -- --test-threads=1"

echo "Tests completed."
