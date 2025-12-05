# Linux

This guide covers how to setup Kawari on Linux.

## Requirements

* Legally obtained copy of the game that's updated to a supported version
* Oodle Network Compression
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). Download the "gcc.zip" file.

## Downloading Kawari

Download the [latest build for Linux from xiv.zone](https://xiv.zone/distrib/kawari/Kawari-Linux.zip) (or pick a specific build from [Github Actions](https://github.com/redstrate/Kawari/actions).)

## Setup

Place the `oodle-network-shared.so` from the Oodle zip you downloaded next to the Kawari executables.

## Reverse proxy setup

{{#include reverse_proxy.md}}

If you get a "permission denied" error starting Caddy, you must either start Caddy with elevated privileges (`sudo`) or set the `CAP_NET_BIND_SERVICE` capability. See [here](https://caddyserver.com/docs/quick-starts/caddyfile) for more information on how to do this.

## Configuration

{{#include configuration.md}}

## Running

Run the server by executing `kawari-run` in your terminal emulator.

## Logging in

{{#include logging_in.md}}
