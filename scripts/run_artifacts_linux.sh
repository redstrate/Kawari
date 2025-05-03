#!/bin/sh

trap 'kill $(jobs -p)' EXIT

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
