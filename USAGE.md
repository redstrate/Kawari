# USage

Kawari is designed to be easy to run, with the goal of being accessible to anyone who wants to run a local server for themselves.

## Copyright Notice

**Kawari requires that you have an active subscription**, and are in possession of a legitimate copy of the game. Kawari is not related or affiliated to SqEx, and by using it you are in explicit violation of the User Agreement (_Limitation 2.4_.)

## Prerequisites
* Linux
* Copy of the game updated to the supported game version (see README)
* Oodle (can be obtained from [here](https://github.com/WorkingRobot/OodleUE))

## Setup

Build Kawari with `cargo build`. Then run it with the helper script:

```shell
$ ./run.sh
```

## Reverse proxy setup

Kawari is useless if it's not behind a domain or other address accessible to a launcher. Even something like Caddy is good enough, and we provide an example setup in the root of the repository.

```shell
# caddy run
```

## Logging in

[Astra](https://github.com/redstrate/Astra) is the only launcher known to work, and it requires compiling the unreleased master branch. **If you don't know what any of that means, then wait for a new release of Astra before trying Kawari.**

1. Enable "Developer Settings" under "General".
2. Under "Developer Settings", enter the addresses of your servers in the section indicated below. If you used the default Caddy setup, tapping the "Set to localhost" button will fill these fields for you with the correct addresses.
3. In "Game Server" and "Game Server Port", set it to "127.0.0.1" and "7000" respectively. This is the address and port of your **Lobby** server.

Any username and password combination will work, as there is no actual login database yet. In the client, make sure to select the **Aether data center**.
