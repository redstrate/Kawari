#!/bin/sh

trap 'kill $(jobs -p)' EXIT
export LD_LIBRARY_PATH=.

./kawari-admin &
./kawari-frontier &
./kawari-login &
./kawari-patch &
./kawari-web &
./kawari-lobby &
./kawari-world &
./kawari-launcher &
./kawari-savedatabank &
wait
