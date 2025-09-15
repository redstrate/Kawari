#!/bin/sh

trap 'kill $(jobs -p)' EXIT
export RUST_BACKTRACE=1

cargo run -q --package kawari --bin kawari-admin &
cargo run -q --package kawari --bin kawari-frontier &
cargo run -q --package kawari --bin kawari-login &
cargo run -q --package kawari --bin kawari-patch &
cargo run -q --package kawari --bin kawari-web &
cargo run -q --package kawari --bin kawari-lobby &
cargo run -q --package kawari --bin kawari-world &
cargo run -q --package kawari --bin kawari-launcher &
cargo run -q --package kawari --bin kawari-savedatabank &
cargo run -q --package kawari --bin kawari-datacentertravel &
wait
