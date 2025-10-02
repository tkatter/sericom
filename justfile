run:
  cargo r -- /dev/ttyUSB0

run-file:
  cargo r -- /dev/ttyUSB0 -f

run-trace:
  cargo r -- /dev/ttyUSB0 -d

check-win:
  cargo c --target x86_64-pc-windows-msvc

test:
  cargo t -p sericom-core --lib
