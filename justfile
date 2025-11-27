alias l := lint
alias c := check
alias t := test
alias tt := test-trace
alias b := build
alias br := build-release

default-trace := 'sericom-core'
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

test tests=default-tests:
  cargo test {{tests}} --no-fail-fast --lib

test-trace target=default-trace tests=default-tests:
  RUST_LOG='{{target}}=trace' cargo test {{tests}} --no-fail-fast --lib -- --nocapture
