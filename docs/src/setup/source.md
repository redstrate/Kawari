# Local Development

This guide covers how to set up a local development environment.

## Requirements

* Legally obtained copy of the game that's updated to a supported version.
* Oodle Network Compression
    * See the requirements for [Windows](windows.md#requirements), [macOS](macos.md#requirements) or [Linux](linux.md#requirements) find the right library for your platform.
* Git
* Rust toolchain
* C/C++ development tools such as CMake (for the few C/C++ libraries we use)

Clone the Kawari repository:

```shell
git clone https://github.com/redstrate/Kawari.git
```

Place the Oodle library for your platform in a new directory called `oodle` inside of the `Kawari` directory.

## Reverse proxy setup

{{#include reverse_proxy.md}}

## Configuration

{{#include configuration.md}}

## Running

You can compile and run the servers with `cargo run`. You can also run a specific server individually with `cargo run --bin`.

## Logging in

{{#include logging_in.md}}
