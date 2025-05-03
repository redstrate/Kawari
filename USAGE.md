# Usage

Kawari is designed to be easy to run, and should be accessible to anyone who wants to run a local server for themselves.

**Note:** Persisted data (logins, characters, etc) are expected to _not_ be stable or secure. Treat all data as disposable.

## Copyright Notice

**Kawari requires that you have an active subscription**, and are in possession of a legitimate copy of the game. Kawari is not related or affiliated to SqEx, and by using it you are in explicit violation of the User Agreement (_Limitation 2.4_.)

## Prerequisites

* Windows or Linux
* Copy of the game updated to the supported game version (see README)
* Oodle Network Compression (can be obtained from [here](https://github.com/WorkingRobot/OodleUE))

## Download Artifact

Windows and Linux artifacts are built on every commit, under [Github Actions](https://github.com/redstrate/Kawari/actions). You will have to download the Oodle Network `.dll` (Windows) or `.so` (Linux) yourself however.

## Building

Build Kawari with `cargo build`.

For the World server to function, Kawari needs to be built with `--features oodle`. Place the `.so` (Linux) or `.lib` (Windows) into the `oodle` directory when building. The library must be named "oodle-network-shared".

## Setup

Afterwards, create a `config.yaml` in the current directory. Currently the minimal config you need to run most services looks like this:

```yaml
game_location: /path/to/gamedir/
```

More configuration options can be found in `config.rs`, such as changing the ports services run on. If you plan on just running it locally for yourself, you don't need to set anything else.

Kawari is made up of multiple executables, to simplify running we have a script to start all of them.

### Windows

Double-click `run.bat`.

### Linux

Run `run.sh` in a terminal.

### Development

Run `scripts/run.sh`.

## Reverse proxy setup

Kawari isn't very useful unless it's addressable to a launcher. We provide a sample Caddyfile reverse proxy setup for this:

```shell
$ caddy run --config resources/Caddyfile
```

This Caddyfile hosts several domains required for normal operation, most notably `ffxiv.localhost`, on port 80. If you get a "permission denied" error starting Caddy, you must either start Caddy with elevated privileges (`sudo`) or set the `CAP_NET_BIND_SERVICE` capability. See [here](https://caddyserver.com/docs/quick-starts/caddyfile) for more information on how to do this.

## Logging in

Navigate to [http://ffxiv.localhost](http://ffxiv.localhost), and register for an account. In order to actually log in, navigate to the Setup page and follow the instructions there.

## Importing characters from retail

It's possible to import existing characters from the retail server using [Auracite](https://auracite.xiv.zone). Upload the backup ZIP on the account management page on the login server.

This feature is still a work-in-progress, and not all data is imported yet.

## Chat commands

### Debug commands

These special debug commands start with `!` and are custom to Kawari.

* `!setpos <x> <y> <z>`: Teleport to the specified location
* `!spawnplayer`: Spawn another player for debugging, not known to work at the moment
* `!spawnnpc`: Spawn a NPC for debugging
* `!spawnmonster`: Spawn a monster for debugging
* `!playscene <id>`: Plays an event. Only some events are supported for now:
    * Territory `181`, Event `1245185` plays the Limsa opening sequence
    * Territory `182`, Event `1245187` plays the Ul'dah opening sequence
    * Territory `183`, Event `1245186` plays the Gridania opening sequence
* `!spawnclone`: Spawn a clone of yourself
* `!classjob <id>`: Changes to another class/job
    
### GM commands

These GM commands are implemented in the FFXIV protocol, but only some of them are implemented.

* `//gm teri <id>`: Changes to the specified territory
* `//gm weather <id>`: Changes the weather
* `//gm wireframe`: Toggle wireframe rendering for the environment
* `//gm item <id>`: Gives yourself an item. This can only place a single item in the first page of your inventory currently.
* `//gm lv <level>`: Sets your current level
