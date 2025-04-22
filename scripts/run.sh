#!/bin/sh

trap 'kill $(jobs -p)' EXIT

cargo run -q --package kawari --features oodle --bin kawari-admin &
cargo run -q --package kawari --features oodle --bin kawari-frontier &
cargo run -q --package kawari --features oodle --bin kawari-login &
cargo run -q --package kawari --features oodle --bin kawari-patch &
cargo run -q --package kawari --features oodle --bin kawari-web &
cargo run -q --package kawari --features oodle --bin kawari-lobby &
cargo run -q --package kawari --features oodle --bin kawari-world &
cargo run -q --package kawari --features oodle --bin kawari-launcher &
wait
