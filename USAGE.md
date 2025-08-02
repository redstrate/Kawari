# Usage

Kawari is designed to be easy to run, and should be accessible to anyone who wants to run a local server for themselves.

**Note:** Persisted data (logins, characters, etc) are expected to _not_ be stable or secure. Treat all data as disposable.

## Copyright Notice

**Kawari requires that you have an active subscription**, and are in possession of a legitimate copy of the game. Kawari is not related or affiliated to SqEx, and by using it you are in explicit violation of the User Agreement (_Limitation 2.4_.)

## Prerequisites

* Windows or Linux
* Legally obtained copy of a supported version of the game (see [README](README.md))
* Oodle Network Compression library
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). Download "gcc.zip" on Linux, and "msvc.zip" on Windows.

## Downloading

We have two ways to download pre-built versions of Kawari, either from xiv.zone or GitHub Actions. You have a choice, depending on your situation:

* xiv.zone only has the latest commit from master, and downloads are accessible to anyone from anywhere.
* GitHub Actions allows you to download builds of specific commits, but this requires a GitHub account. (These builds also expire after 90 days.)

Here is the [latest build for Windows from xiv.zone](https://xiv.zone/distrib/kawari/Kawari-Windows.zip) and the [latest build for Linux](https://xiv.zone/distrib/kawari/Kawari-Linux.zip). For [Github Actions](https://github.com/redstrate/Kawari/actions), look under the Summary tab to download a build.

## Building

Building Kawari is simple, run `cargo build` in a terminal.

For the World server to function, Kawari needs to be built with `--features oodle`. On Linux, place the `oodle-network-shared.so` in a folder created by you named `oodle` before building. On Windows, no extra step is necessary here.

## Setup

Place the `oodle-network-shared.dll` (Windows) or `oodle-network-shared.so` (Linux) in the same directory as the executables. If you skip this step, the World server cannot function.

Then create a `config.yaml` in the current directory. At a minimum, you have to specify the path to the game directory:

```yaml
filesystem:
    game_path: C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game
```

More configuration options can be found in `config.rs`, such as changing the ports services run on. If you plan on just running it locally for yourself, you can begin running the servers.

## Running

Since Kawari is made up of multiple executables, we have included a built-in script to start them all at once.

* On Windows, double-click `run.bat`.
* On Linux, run `run.sh` in a terminal.
* For developers, run `scripts/run.sh` in a terminal.

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
* `!unlock <id>`: Unlock an action, emote, etc. for example: `1` for Return and `4` for Teleport.
* `!equip <name>`: Forcefully equip an item, useful for bypassing class/job and other client restrictions. This will *overwrite* any item in that slot!
* `!nudge <distance> <up/down (optional)>`: Teleport forward, back, up or down `distance` yalms. Specifying up or down will move the player up or down instead of forward or back. Examples: `!nudge 5 up` to move up 5 yalms, `!nudge 5` to move forward 5 yalms, `!nudge -5` to move backward 5 yalms.
* `!festival <id1> <id2> <id3> <id4>`: Sets the festival in the current zone. Multiple festivals can be set together to create interesting effects.
* `!reload`: Reloads `Global.lua` that is normally only loaded once at start-up.
* `!finishevent`: Forcefully finishes the current event, useful if the script has an error and you're stuck talking to something.
* `!item <name>`: Gives you an item matching by name.
* `!inspect`: Prints info about the player.
* `!completeallquests`: Completes every quest in the game, useful for accessing stuff gated behind quest completion.
* `!unlockcontent <id>`: Unlocks the specified instanced content.
* `!replay <path>`: Replays packets, must be in the format generated from cfcap-capture.
* `!condition <name>`: Forcefully sets a condition, see `condition.rs` for what is supported.
* `!clearconditions`: Forcefully clears all conditions set on your character.

### GM commands

These GM commands are implemented in the FFXIV protocol, but only some of them are implemented.

* `//gm teri <id>`: Changes to the specified territory
* `//gm weather <id>`: Changes the weather
* `//gm wireframe`: Toggle wireframe rendering for the environment
* `//gm item <id>`: Gives yourself an item. This can only place a single item in the first page of your inventory currently.
* `//gm lv <level>`: Sets your current level
* `//gm aetheryte <on/off> <id>`: Unlock an Aetheryte.
* `//gm speed <multiplier>`: Increases your movement speed by `multiplier`.
* `//gm orchestrion <on/off> <id>`: Unlock an Orchestrion song.
* `//gm exp <amount>`: Adds the specified amount of EXP to the current class/job.
* `//gm teri_info`: Displays information about the current zone. Currently displays zone id, weather, internal zone name, parent region name, and place/display name.
* `//gm gil <amount>`: Adds the specified amount of gil to the player
* `//gm collect <amount>`: Subtracts `amount` gil from the targeted player (yourself only for now).
