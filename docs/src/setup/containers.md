# Linux

This guide covers how to setup Kawari with containers (Podman/Docker.) Despite our container being based on Linux, it can be used on any platform.

## Requirements

* Podman or Docker installed
* Legally obtained copy of the game that's updated to a supported version
* Oodle Network Compression
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). Download the "gcc-release.zip" file.

## Building Image

The container image must be manually built, because of the non-redistributable Oodle requirement. Place your copy of Oodle in a new `oodle` folder before continuing.

If you're using Podman run: 

```shell
podman build -t kawari .
```

Or else, with Docker:

```shell
docker build -t kawari .
```

## Reverse proxy setup

{{#include reverse_proxy.md}}

If you get a "permission denied" error starting Caddy, you must either start Caddy with elevated privileges (`sudo`) or set the `CAP_NET_BIND_SERVICE` capability. See [here](https://caddyserver.com/docs/quick-starts/caddyfile) for more information on how to do this.

## Configuration

{{#include configuration.md}}

## Running

Run the server by executing the container by your preferred method.

## Logging in

{{#include logging_in.md}}
