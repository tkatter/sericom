alias l := lint
alias c := check
alias t := test
alias b := build
alias br := build-release

default-tests := ''

default: check lint

build:
  cargo build

build-release:
  cargo build --release

check:
  cargo check

check-win:
  cargo check --target x86_64-pc-windows-msvc

clean:
  cargo clean

lint:
  cargo clippy

list-tests:
  cargo test -- --list

run:
  cargo run -- /dev/ttyUSB0

run-file:
  cargo run -- /dev/ttyUSB0 -f

testseri:
  cargo run -p test-sericom

run-trace:
  cargo run -- /dev/ttyUSB0 -d

test target=default-tests:
  cargo test {{target}} --no-fail-fast --lib
