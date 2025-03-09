#!/bin/sh

trap 'kill $(jobs -p)' EXIT

cargo run -q --package kawari --bin kawari-admin &
cargo run -q --package kawari --bin kawari-frontier &
cargo run -q --package kawari --bin kawari-login &
cargo run -q --package kawari --bin kawari-patch &
cargo run -q --package kawari --bin kawari-web &
cargo run -q --package kawari --bin kawari-lobby &
cargo run -q --package kawari --bin kawari-world &
wait
