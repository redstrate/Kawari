# Kawari

Server emulator for a certain MMO. Updates on the project can be found [on my blog](https://redstrate.com/blog/series/kawari-progress-report/).

## Features

We have a working implementation for most of FFXIV's servers:

* Frontier
    * Used for news, gate status and more.
* Launcher
    * Used to serve the launcher web page.
* Lobby
    * Create a new character and login to a World server.
* Login
    * Logging in and creating new accounts.
* Patch
    * Verifies the game client version. Can't serve patch files yet.
* World
    * Still limited, but supports basic multiplayer and can explore zones.

## Goals

Kawari is primarily a research project, but also a way to preserve the modern client. Kawari is...
* **FOR** preservation of the game, in the event that the servers are permanently unavailable.
* **FOR** exploring the packet structure for legitimate purposes (e.g. archival and preservation.)
* **NOT** a way to play the game without a valid subscription.
* **NOT** for creating bots, packet modifications or doing anything on the retail servers.

## Supported Game Version

Kawari currently supports patch **7.25** (2025.05.17.0000.0000.) Kawari will never "roll back" to a previous patch. There are other servers (e.g. Sapphire) that support older versions of the game. As Kawari moves to a new major patch, the previous patch is moved to a branch (e.g. 7.1) These branches are for archival: effectively unsupported, but still useful.

Only the Global region is supported. Only the Windows client is supported. Supporting other regions or clients are currently out of scope of this project, but might work anyway.

## Running

Kawari is designed to be easy to run. A guide to running Kawari can be followed [here](USAGE.md).

## Contributing

Pull requests for new features, patch updates, and documentation are welcome. A guide for contributing and updating Kawari can be found [here](CONTRIBUTING.md).

## Credits & Thank You

* [Sapphire](https://github.com/SapphireServer/Sapphire) for reference.
* [iolite](https://github.com/0xbbadbeef/iolite) for inspiration & reference.
* [TemporalStasis](https://github.com/NotNite/TemporalStasis) for tooling and reference.
* [Project Chronofoil](https://github.com/ProjectChronofoil/) for easy packet capture.
* [FFXIVClientStructs](https://github.com/aers/FFXIVClientStructs/) for being an invaluable resource for the client's internals.

## License

This project is licensed under the [GNU Affero General Public License 3](LICENSE). Some code or assets may be licensed differently.
