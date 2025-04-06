# Usage

Kawari is designed to be easy to run, with the goal of being accessible to anyone who wants to run a local server for themselves.

**Note:** Persisted data (logins, characters, etc) is _not_ stable or secure. Treat all data as disposable.

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
game_location: pathtogamedir
```

More configuration options can be found in `config.rs`, such as changing the ports services run on. Finally, run Kawari with the helper script:

```shell
$ ./scripts/run.sh
```

## Reverse proxy setup

Kawari is useless if it's not behind a domain or other address accessible to a launcher. Even something like Caddy is good enough, and we provide an example setup in the root of the repository.

```shell
$ caddy run resources/Caddyfile
```

This Caddyfile hosts several domains, most notably `ffxiv.localhost`, on port 80. If you get a "permission denied" error starting Caddy, you must either start Caddy with elevated privileges or set the `CAP_NET_BIND_SERVICE` capability. See [here](https://caddyserver.com/docs/quick-starts/caddyfile) for more information on how to do this.

## Logging in

### Astra

[Astra](https://github.com/redstrate/Astra) is the only launcher known to fully implement the login process, and it requires compiling the unreleased master branch. **If you don't know what any of that means, then wait for a new release of Astra before trying Kawari.**

1. Enable "Developer Settings" under "General".
2. Under "Developer Settings", enter the addresses of your servers in the section indicated below. If you used the default Caddy setup, tapping the "Set to localhost" button will fill these fields for you with the correct addresses.
3. In "Game Server" and "Game Server Port", set it to "127.0.0.1" and "7000" respectively. This is the address and port of your **Lobby** server.

Any username and password combination will work, as there is no actual login database yet. In the client, make sure to select the **Aether data center**.

### Manual

Advanced users can specify required command line arguments directly to the game executable. This skips most of the Kawari login process and should only be used if you know what you're doing. Right now, the lobby server does not check for authentication, but in the future you must complete the login process manually to get a valid session ID.

In this example, lobby number 4 will replace the **Aether data center**, but the other data centers will still try (and fail) to connect.

* `DEV.LobbyHost04=127.0.0.1`
* `DEV.LobbyPort04=7000`
* `DEV.TestSID=0` (this must be a valid session ID in the future)

Some other launchers (like XIVLauncher) will allow you to specify these extra arguments, but they will still authenticate to the retail servers. You can still connect to Kawari with this way, but **make sure to specify your own session ID, or your retail account's session ID will be sent to the lobby server**!

## Importing characters from retail

It's possible to import existing characters from the retail server using [Auracite](https://auracite.xiv.zone). Place the backup ZIP under `backups/` and the World server will import it into the database the next time it's started.

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
