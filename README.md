[![Crates.io Version](https://img.shields.io/crates/v/sericom?style=for-the-badge&color=green)](https://crates.io/crates/sericom)

# Sericom

This repository houses the **Sericom CLI application**, a tool for communicating
with devices over serial connection, and its underlying library [**Sericom-core**](https://github.com/tkatter/sericom/blob/main/sericom-core/README.md).

- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Keymaps](#keymaps)
- [Roadmap](#roadmap)
- [License](#license)

## What is Sericom?

Sericom is a CLI tool for communicating with devices over a serial connection.

Currently, it functions similar to the `screen` CLI tool (when used to communicate
over a serial connection). It will print the lines received from a device over a
serial connection to the terminal's screen and send commands to the device as it
receives them from stdin.

Sericom has the following basic functionality (tested on Windows and Linux):

- Scrolling through a session's history
- Selecting text via the mouse and copying it to the clipboard
- Pasting text into stdin (<kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>v</kbd>)
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

### With Cargo from crates.io

To install Sericom from crates.io, you will need to have Rust and Rust's package
manager Cargo installed (if you have a standard installation of Rust, you should
also have Cargo installed).

To check that Rust and Cargo are installed, run the following commands:

```
rustc --version
cargo --version
```

To install Sericom from crates.io, run:

```
cargo install sericom
```

Sericom will be installed in `$HOME/.cargo/bin/`. As long as that directory is
in your `PATH`, you should be able to start using Sericom!

### Debian/Ubuntu Binary `.deb` File Installation

If using Debian or a Debian derivative like Ubuntu, Sericom can be installed
using a binary `.deb` file provided in [Sericom's releases](https://github.com/tkatter/sericom/releases/).

Either navigate to the [release](https://github.com/tkatter/sericom/releases/) page and download the appropiate file or run:

```
curl -LO https://github.com/tkatter/sericom/releases/download/v0.2.0/sericom_0.2.0-1_amd64.deb

# Once downloaded, install with the apt package manager
sudo apt install ./<path_to_downloaded_release>.deb

# Verify installation
sericom --version

# Check the sha256 checksum
curl -LO https://github.com/tkatter/sericom/releases/download/v0.2.0/sericom_0.2.0-1_amd64.deb
curl -LO https://github.com/tkatter/sericom/releases/download/v0.2.0/sericom_0.2.0-1_amd64.deb.sha256

sha256sum -c sericom_0.2.0-1_amd64.deb.sha256
```

### Build From Source

In order to build from source, you will need to have Rust and Cargo installed
([Rust's installation instructions](https://www.rust-lang.org/tools/install)).

Clone the repository and build with Cargo in the main branch:

```
git clone https://github.com/tkatter/sericom.git
cd sericom
cargo build --release
```

Cargo will build the binary and put it in `sericom/target/release/sericom`.

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

- Scroll to the top of the session's history: <kbd>F1</kbd>
- Scroll to the bottom of the session's history: <kbd>F2</kbd>
- Copy text: simply select the text with your mouse; upon releasing the mouse button, the selected text will be automatically copied to your clipboard
- Paste text: <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>v</kbd>
- Clear the screen and clear the session's history: <kbd>Ctrl</kbd> + <kbd>l</kbd>
- Quit: <kbd>Ctrl</kbd> + <kbd>q</kbd>

### Configuration

Sericom can be configured via a `config.toml` file. Sericom looks for this file
in the `$HOME/.config/sericom/` directory. You may specify as little or as many
configurations as you'd like, for any that are not defined in your config, Sericom
will fall back to its [defaults](https://github.com/tkatter/sericom/blob/main/configuration/defaults.toml).

Currently the available configuration options are as follows:

```toml
[appearance]
# Control the foreground and background colors
fg = "green"
bg = "none"

# Control the foreground and background colors of highlighted text
# (text that is highlighted as it is selected via the mouse)
hl_fg = "black"
hl_bg = "white"

[defaults]
# The default directory where files will be written to when running `sericom -f <FILE>`
out_dir = "./"
```

> [!NOTE] Behavior of the `out_dir` configuration
> When using a _relative path_ with the `-f` flag, Sericom will recursively create
> the file within whatever is set as the `out_dir` (by default it is the current directory
> where sericom was run (`"./"`)).
>
> When using an _absolute path_ with the `-f` flag, Sericom will ignore the `out_dir` config
> value and recursively create the file at the location of the absolute path.
>
> **Examples**
>
> ```bash
> # Using the default value of `out_dir`, which is ./ (current directory)
> $ pwd
> # /home/thomas/device_files
> $ sericom -f c2960.txt
> # file is created at /home/thomas/device_files/c2960.txt
> $ sericom -f tests/c2960.txt
> # file is created at /home/thomas/device_files/tests/c2960.txt
> $ sericom -f ../c2960.txt
> # file is created at /home/thomas/c2960.txt
> $ sericom -f /home/thomas/other_tests/c2960.txt
> # file is created at /home/thomas/other_tests/c2960.txt
> $ sericom -f $HOME/c2960.txt
> # file is created at /home/thomas/c2960.txt
> ```

A list of all the available options can be found [here](https://github.com/tkatter/sericom/blob/main/configuration/values.md).

If there are additional configuration options you would like to see added, please open an issue!

## Roadmap

- [x] Allow users to write a TOML configuration file. The file should allow the configuration of the following:
  - [x] Appearance settings (color of text/background)
  - [ ] The maximum number of lines that are stored in the program's history
  - [x] Default directory for files to be written to
  - [ ] Keymaps
- [ ] Create a simple scripting language for users to write scripts (similar to expect scripts). Purposes of the scripts:
  - [ ] Send commands to the device
  - [ ] Scan for specific information to use elsewhere.
  - [ ] Expect some output, and based on the output do something (send command, close connection, write ouput to file, etc.)
  - Pseudo-example:
  ```
  send! "show inventory"
  grep! first! "SN:"
    |> get! word! "SN: $1"
    |> set! SN=$1
    |> writef_ln! "Serial number: '$SN'"
  ```
- [ ] Allow users to create config files that describe specific devices/software/operating systems. Purposes of the config files:
  - [ ] Define a device (name, model, brand, operating system)
  - [ ] Define a software version (Cisco IOS, NX-OS, Dell OS9, etc.)
  - [ ] Definitions of devices/software versions would include lines/identifiers that Sericom can scan for as lines are received from the serial connection.
  - [ ] These definitions would serve to identify a device and then perform user-defined tasks/scripts. Examples could be to configure a switch, reset a device, run tests and write the results to a file.

## Linux Setup

Need to add user to the correct group to avoid getting `Permission Denied` errors.  
Use `ls -l /dev/tty*` to check the group of the serial ports - in my case it was `dialout`.  
Add user to the `dialout` group with `sudo usermod -a -G dialout $USER`.

## License

This program, Sericom, is licensed under the [GNU GPL v3.0](https://github.com/tkatter/sericom/blob/main/sericom/LICENSE).

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

---

The `sericom-core` library is released under a [MIT license](https://github.com/tkatter/sericom/blob/main/sericom-core/LICENSE).
