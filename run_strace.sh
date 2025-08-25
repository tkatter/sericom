#!/usr/bin/bash

OPTIONS=""

if [ "$1" == "-f" ]; then
  OPTIONS="-f /home/thomas/Code/Work/netcon/output.txt"
fi

/usr/bin/strace -f -c netcon /dev/ttyUSB0 $OPTIONS

# Check write patterns - analyze writes.log
# /usr/bin/strace -e write -o writes.log netcon /dev/ttyUSB0 -f /home/thomas/Code/Work/netcon/output.txt
