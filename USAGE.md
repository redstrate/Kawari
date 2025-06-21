# Usage

Kawari is designed to be easy to run, and should be accessible to anyone who wants to run a local server for themselves.

**Note:** Persisted data (logins, characters, etc) are expected to _not_ be stable or secure. Treat all data as disposable.

## Copyright Notice

**Kawari requires that you have an active subscription**, and are in possession of a legitimate copy of the game. Kawari is not related or affiliated to SqEx, and by using it you are in explicit violation of the User Agreement (_Limitation 2.4_.)

## Prerequisites

* Windows or Linux
* Copy of the game updated to the supported game version (see [README](README.md))
* Oodle Network Compression (see below)

### Getting Oodle

Get the [latest release from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). You want the "gcc.zip" for Linux, and "msvc.zip" for Windows.

## Download Artifact

Windows and Linux artifacts are built on every commit, under [Github Actions](https://github.com/redstrate/Kawari/actions). Place the `oodle-network-shared.dll` (Windows) or `oodle-network-shared.so` (Linux) in the same directory as the executables.

## Building

Build Kawari with `cargo build`.

For the World server to function, Kawari needs to be built with `--features oodle`. Place the `oodle-network-shared.lib` (Windows) or `oodle-network-shared.so` (Linux) in a folder created by you named `oodle` before building.

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

Run `scripts/run.sh` in a terminal.

## Reverse proxy setup

Kawari isn't very useful unless it's addressable to a launcher, so we have to setup a "reverse proxy". We suggest using [Caddy](https://caddyserver.com/download) and we also have a configuration that works on most local setups. Run this in your operating system's terminal. If you're on Windows, point it to the Caddy `.exe`.

```shell
caddy run --config resources/Caddyfile
```

This Caddyfile hosts several domains required for normal operation, for example `ffxiv.localhost` on port 80. If you get a "permission denied" error starting Caddy, you must either start Caddy with elevated privileges (`sudo`) or set the `CAP_NET_BIND_SERVICE` capability. See [here](https://caddyserver.com/docs/quick-starts/caddyfile) for more information on how to do this.

You'll notice that your browser can connect to the `.localhost` sites without any additional configuration, e.g. `ffxiv.localhost`. Whatever magic Caddy does to make this happen _only works in browsers_, so the WinHTTP-based code in FFXIV will fail. To work around this, you will have to edit your hosts file:

```
127.0.0.1 admin.ffxiv.localhost
127.0.0.1 ffxiv.localhost
127.0.0.1 launcher.ffxiv.localhost
127.0.0.1 config-dl.ffxiv.localhost
127.0.0.1 frontier.ffxiv.localhost
127.0.0.1 patch-bootver.ffxiv.localhost
127.0.0.1 patch-gamever.ffxiv.localhost
127.0.0.1 ffxiv-login.square.localhost
127.0.0.1 patch-dl.ffxiv.localhost
```

On Windows this file is located under `C:\Windows\System32\Drivers\etc\hosts` and on Linux it's located under `/etc/hosts`. **If you plan on using Astra to connect to Kawari, this is not needed.**

## Logging in

Navigate to [http://ffxiv.localhost](http://ffxiv.localhost), and register for an account. In order to actually log in, navigate to the Setup page and follow the instructions there. If you get an error in your web browser, ensure you're connecting via **http://** and not **https://**.

By default, the World server advertises itself as Gilgamesh but this can be changed in `config.yaml`:

```yaml
world:
    world_id: 63
```

This has no actual effect in-game, apart from the World name shown inside the client. All data centers will show the configured world.

## Importing characters from retail

It's possible to import existing characters from the retail server using [Auracite](https://auracite.xiv.zone). Upload the backup ZIP on the account management page on the login server.

This feature is still a work-in-progress, and not all data is imported yet.

## Chat commands

### Debug commands

These special debug commands start with `!` and are custom to Kawari.

* `!setpos <x> <y> <z>`: Teleport to the specified location
* `!spawnnpc`: Spawn a NPC for debugging
* `!spawnmonster`: Spawn a monster for debugging
* `!spawnclone`: Spawn a clone of yourself
* `!classjob <id>`: Changes to another class/job
* `!unlockaction <id>`: Unlock an action, for example: `1` for Return and `4` for Teleport.
* `!equip <name>`: Forcefully equip an item, useful for bypassing class/job and other client restrictions. This will *overwrite* any item in that slot!
* `!nudge <distance> <up/down (optional)>`: Teleport forward, back, up or down `distance` yalms. Specifying up or down will move the player up or down instead of forward or back. Examples: `!nudge 5 up` to move up 5 yalms, `!nudge 5` to move forward 5 yalms, `!nudge -5` to move backward 5 yalms.
* `!festival <id1> <id2> <id3> <id4>`: Sets the festival in the current zone. Multiple festivals can be set together to create interesting effects.
* `!reload`: Reloads `Global.lua` that is normally only loaded once at start-up.
* `!wireframe: Toggles the hidden GM wireframe rendering mode on or off.`
* `!invis: Toggles the hidden GM invisibility mode on or off for your character.`
* `!unlockaetheryte <on/off> <id>`: Unlock an aetheryte. For the first parameter, the literal words are required.
* `!teri <id>`: Changes to the specified territory.

### GM commands

These GM commands are implemented in the FFXIV protocol, but only some of them are implemented.

* `//gm teri <id>`: Changes to the specified territory
* `//gm weather <id>`: Changes the weather
* `//gm wireframe`: Toggle wireframe rendering for the environment
* `//gm item <id>`: Gives yourself an item. This can only place a single item in the first page of your inventory currently.
* `//gm lv <level>`: Sets your current level
* `//gm aetheryte <on/off> <id>`: Unlock an Aetheryte.
* `//gm speed <multiplier>`: Increases your movement speed by `multiplier`.
