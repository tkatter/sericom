#!/usr/bin/bash

# /usr/bin/perf record --call-graph=dwarf netcon /dev/ttyUSB0 -f /home/thomas/Code/Work/netcon/output.txt

# Check which functions are causing futex calls
/usr/bin/perf record -e syscalls:sys_enter_futex -- /home/thomas/Code/Work/netcon/target/release/netcon /dev/ttyUSB0 -f /home/thomas/Code/Work/netcon/output.txt
# /usr/bin/perf report

# Monitor lock contentions
# /usr/bin/perf record -e lock:lock_contention -g netcon /dev/ttyUSB0 -f /home/thomas/Code/Work/netcon/output.txt
