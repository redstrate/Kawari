# Contributing and working on Kawari

Here are various helpful resources and tips when working on Kawari.

## Packet capture

The well-tested packet capturing solutions are [TemporalStasis](https://github.com/WorkingRobot/TemporalStasis/) and [Project Chronofoil](https://github.com/ProjectChronofoil). You should use Project Chronofoil under most circumstances, but it requires Dalamud. TemporalStasis works like a standalone proxy server, and can work with a vanilla game.

To extract `.cfcap` captures from Project Chronofoil, use `cfcap-expand` from [XIVPacketTools](https://github.com/redstrate/XIVPacketTools).

## Updating to new patches

Here are the various things that should be checked when updating Kawari to a newer patch:

* Bump the supported game versions in `lib.rs` so the patch server lets you through.
* Double check IPC struct sizes in `calc_size()` if the structures changed.
* Replace testing data in `resources/tests` and re-run the tests.
* The IPC opcodes _will_ change and must all be replaced.
* Check the game version used in the encryption key in `lib.rs`.

## IPC Opcodes

Since the Zone IPC opcodes change every patch, it's extremely easy to change the opcodes in Kawari. Edit the values under `resources/opcodes.json` and recompile Kawari. You still have to change the structs themselves (located under `src/<connection>/ipc`) if needed though.

## Contributing

Before making a pull request, make sure:

* Kawari compiles and runs fine. At a minimum, you should be able to login to the World server.
* Run `cargo fmt` to ensure your code is formatted.
* Run `cargo clippy` and fix all of the warnings for any new code, to the best of your ability.
