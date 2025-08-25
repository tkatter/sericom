#!/usr/bin/bash

# Add these to `Cargo.toml`
# tokio = { .. features = ["full", "tracing"] }
# tracing = "0.1"
# console-subscriber = "0.4.1"

RUSTFLAGS="--cfg tokio_unstable" /home/thomas/.cargo/bin/cargo run -- /dev/ttyUSB0 -f /home/thomas/Code/Work/netcon/output.txt
