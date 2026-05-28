# Linux

This guide covers how to setup Kawari on Linux.

## Requirements

* Legally obtained copy of the game that's updated to a supported version
* Oodle Network Compression
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). Download the "gcc-x64-release.zip" or "gcc-arm64-release" file depending on your architecture.

## Downloading Kawari

Download the [latest build for Linux x86_64](https://xiv.zone/distrib/kawari/kawari-linux-x86_64.zip) or [ARM64](https://xiv.zone/distrib/kawari/kawari-linux-aarch64.zip) (or pick a specific build from [Github Actions](https://github.com/redstrate/Kawari/actions).)

## Setup

Place the `oodle-network-shared.so` from the Oodle zip you downloaded next to the Kawari executables.

## Configuration

{{#include configuration.md}}

## Hosts setup

{{#include hosts.md}}

## Running

Run the server by executing `kawari-run` in your terminal emulator.

## Logging in

{{#include logging_in.md}}
