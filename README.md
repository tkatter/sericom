# Sericom

Sericom is a CLI tool for communicating with devices over a serial connection.

Currently, it functions similar to the `screen` CLI tool (when used to communicate 
over a serial connection). It will print the lines received from a device over a
serial connection to the terminal's screen and send commands to the device as it
receives them from stdin.

Sericom has the following basic functionality (tested on Windows and Linux):
- Scrolling through a session's history
- Selecting text via the mouse and copying it to the clipboard 
- Pasting text into stdin (<kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>v<kbd>)
- Writing the lines received to a file

The goal of Sericom, going forward, is to serve as an automation tool to execute
tasks on devices via a serial connection. Sericom was originally developed with the
intention of communicating with networking devices (switches, routers, firewalls, etc),
and automating the process of tasks like: running tests and saving the results to a file,
performing a configuration reset, configuring a device, etc. Even though Sericom was
initially developed to be used with networking devices, the intention going forward is to
be compatible (from an automation standpoint) with most devices. On that note, if you encounter
an issue or problem, please open a Github issue.

## Installation

TODO!

## Usage

### Basic usage

- To open a connection to a serial port (uses a default baud rate of 9600):
```
# Syntax
sericom <PORT>

# Windows
sericom.exe COM4

# Linux
sericom /dev/ttyUSB0
```
- To open a connection and write everything received to a file:
```
# Syntax
sericom -f <PATH_TO_FILE> <PORT> 
```
- To get a list of all the valid baud rates:
```
sericom list-bauds
```
- To see all of the available serial ports:
```
sericom list-ports
```

### Keymaps

- Scroll to the top of the session's history: <kbd>F1<kbd>
- Scroll to the bottom of the session's history: <kbd>F2<kbd>
- Copy text: simply select the text with your mouse; upon releasing the mouse button, the selected text will be automatically copied to your clipboard
- Paste text: <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>v<kbd>
- Clear the screen and clear the session's history: <kbd>Ctrl<kbd> + <kbd>l<kbd>
- Quit: <kbd>Ctrl<kbd> + <kbd>q<kbd>

## Roadmap

- Allow users to write a TOML configuration file. The file should allow the configuration of the following:
    - Appearance settings (color of text/background)
    - The maximum number of lines that are stored in the program's history
    - Default directory for files to be written to
    - Keymaps
- Create a simple scripting language for users to write scripts (similar to expect scripts). Purposes of the scripts:
    - Send commands to the device
    - Scan for specific information to use elsewhere.
    - Expect some output, and based on the output do something (send command, close connection, write ouput to file, etc.)
    - Pseudo-example:
    ```
    send! "show inventory"
    grep! first! "SN:"
      |> get! word! "SN: $1"
      |> set! SN=$1
      |> writef_ln! "Serial number: '$SN'"
    ```
- Allow users to create config files that describe specific devices/software/operating systems. Purposes of the config files:
    - Define a device (name, model, brand, operating system)
    - Define a software version (Cisco IOS, NX-OS, Dell OS9, etc.)
    - Definitions of devices/software versions would include lines/identifiers that Sericom can scan for as lines are received from the serial connection. 
    - These definitions would serve to identify a device and then perform user-defined tasks/scripts. Examples could be to configure a switch, reset a device, run tests and write the results to a file.

## Linux Setup

Need to add user to the correct group to avoid getting `Permission Denied` errors.  
Use `ls -l /dev/tty*` to check the group of the serial ports - in my case it was `dialout`.  
Add user to the `dialout` group with `sudo usermod -a -G dialout $USER`.

## License

This program, Sericom, is licensed under the [GNU GPL v3.0](https://github.com/tkatter/sericom/blob/main/LICENSE).

This program, Sericom, is free software: you can redistribute it and/or modify
it under the terms of the **GNU General Public License as published by
the Free Software Foundation, or (at your option) any later version.**

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

---

**Note:** The author reserves the right to offer this software under a separate
commercial license at a future date for those who wish to use it outside
the terms of the GPLv3.
