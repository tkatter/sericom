#!/usr/bin/bash

# Needed to run this to make `perf` work:
# echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid
/home/thomas/.cargo/bin/cargo flamegraph --bin netcon -- /dev/ttyUSB0 -f /home/thomas/Code/Work/netcon/output.txt
