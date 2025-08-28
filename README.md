# Serial CLI Tool

This tool is purpose built for my work. We receive old networking equipment  
from enterprise manufacturers (i.e. Dell, Cisgo, Juniper) and must clear any  
existing configs and reset the device to it's factory defaults.

This is done via a serial connection to the device's CONSOLE port. I've decided  
to try to build a CLI tool in Rust that can run a set of scripts to help automate  
this process.

## TODO

- [ ] Implement hot-keys like <F1> or <Alt + B> etc.
- [ ] Implement reading data to a buffer and logging while scanning buffer for patterns (errors and info like serial number)
- [ ] Implement config file (location is OS dependent) for controling things like apperance and destination for files
- [ ] Propogate errors related to failing to open a serial connection i.e. port in use

## Linux Setup

Need to add user to the correct group to avoid getting `Permission Denied` errors.  
Use `ls -l /dev/tty*` to check the group of the serial ports - in my case it was `dialout`.  
Add user to the `dialout` group with `sudo usermod -a -G dialout $USER`.

## Error Messages

**Error: Custom { kind: Other, error: "failed to apply some or all settings" }**  
From what I've tested, started getting this error when opening ports with `list-settings`  
and using a baud rate >115200 on Linux Mint.

**Error: Os { code: 5, kind: Uncategorized, message: "Input/output error" }**  
Pretty sure this is because Linux hardcodes `x` amount of serial ports in the kernel.  
The `/dev/tty*` devices are created regardless of the actual number of physical serial  
ports your system has.

I only get this error if I try to use any port other than `/dev/ttyS0` on my machine.  
Therefore, I'm assuming that I only have 1 physical serial port which is why only  
`/dev/ttyS0` responds to this program and the others simply do not exist.

**Error: Os { code: 13, kind: PermissionDenied, message: "Permission denied" }**  
Permission denied, if on Linux - see [Linux Setup](#linux-setup).  
If have already added Linux user to the correct group, unsure what the problem is yet.

**Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }**  
Supplied path to serial port is invalid/doesn't exist.

## Licensing

This program, Sericom, is licensed under the [GNU GPL v3.0]().

This program, Sericom, is free software: you can redistribute it and/or modify
it under the terms of the **GNU General Public License as published by
the Free Software Foundation.**

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

---
**Note:** The author reserves the right to offer this software under a separate
commercial license at a future date for those who wish to use it outside
the terms of the GPLv3.
