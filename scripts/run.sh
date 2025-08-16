#!/bin/sh

trap 'kill $(jobs -p)' EXIT
RUST_BACKTRACE=1

cargo run -q --package kawari --features oodle --bin kawari-admin &
cargo run -q --package kawari --features oodle --bin kawari-frontier &
cargo run -q --package kawari --features oodle --bin kawari-login &
cargo run -q --package kawari --features oodle --bin kawari-patch &
cargo run -q --package kawari --features oodle --bin kawari-web &
cargo run -q --package kawari --features oodle --bin kawari-lobby &
cargo run -q --package kawari --features oodle --bin kawari-world &
cargo run -q --package kawari --features oodle --bin kawari-launcher &
cargo run -q --package kawari --features oodle --bin kawari-savedatabank &
cargo run -q --package kawari --features oodle --bin kawari-datacentertravel &
wait
