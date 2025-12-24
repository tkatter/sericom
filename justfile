alias b := build
alias br := build-release
alias c := check
alias f := fmt
alias l := lint
alias r := run
alias t := test
alias tt := test-trace

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

fmt:
  cargo fmt

lint:
  cargo clippy

list-tests:
  cargo test -- --list

run:
  cargo run -p sericom

run-file:
  cargo run -- /dev/ttyUSB0 -f

testseri:
  cargo run -p test-sericom

run-trace:
  cargo run -- /dev/ttyUSB0 -d

test tests=default-tests:
  cargo test {{tests}} --no-fail-fast --lib

test-trace tests=default-tests:
  RUST_LOG='sericom_core::screen::buffer=debug,sericom_core=trace' cargo test {{tests}} --no-fail-fast --lib -- --nocapture
