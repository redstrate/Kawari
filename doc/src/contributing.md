# Contributing and working on Kawari

Here are various helpful resources and tips when working on Kawari.

## Recommended Dalamud plugins

Here are some Dalamud plugins that will make your life easier:

* [AllowLoginFail](https://codeberg.org/redstrate/AllowLoginFail) to stop the game from rage quitting after hitting lobby errors.
* [Scripter](https://codeberg.org/redstrate/Scripter) for inspecting the client's Lua state.

## Packet capture

The well-tested packet capturing solutions are [TemporalStasis](https://github.com/WorkingRobot/TemporalStasis/) and [Project Chronofoil](https://github.com/ProjectChronofoil). You should use Project Chronofoil under most circumstances, but it requires Dalamud. TemporalStasis works like a standalone proxy server, and can work with a vanilla game.

To extract `.cfcap` captures from Project Chronofoil, use `cfcap-expand` from [XIVPacketTools](https://codeberg.org/redstrate/XIVPacketTools).

## Updating to new patches

Here are the various things that should be checked when updating Kawari to a newer patch:

* Bump the supported game versions in `lib.rs` so the patch server lets you through.
* Double check IPC struct sizes in `calc_size()` if the structures changed.
* Replace testing data in `resources/data/tests` and re-run the tests as needed.
* The IPC opcodes _will_ change and must all be replaced.
* Check and update the various constants in `lib.rs`.
* If the Excel schema changed, update the Icarus version in `Cargo.toml`.
* If [Unscrambler](https://github.com/perchbirdd/Unscrambler/) updated, make sure to copy over the tables in `resources/data/scrambler`.

## IPC Opcodes

Since the Zone IPC opcodes change every patch, it's extremely easy to change the opcodes in Kawari. Edit the values under `resources/data/opcodes.json` and recompile Kawari. You still have to change the structs themselves (located under `src/ipc`) if needed though.

Opcodes can be updated from (from least to most pain): 
* The [opcodediff](https://github.com/xivdev/opcodediff) and the [opcode-update tool from XIVPacketTools](https://codeberg.org/redstrate/XIVPacketTools) once they update.
* The [FFXIVOpcodes repository](https://github.com/karashiiro/FFXIVOpcodes/blob/master/opcodes.json) once they update.
* Manual testing using tools like Chronofoil.

## Contributing

Before making a pull request, make sure:

* Kawari compiles and runs fine. At a minimum, you should be able to login to the World server.
* Run `cargo fmt` to ensure your code is formatted.
* Run `cargo clippy` and fix all of the warnings for any new code, to the best of your ability.

## Testing local Physis

Sometimes, you need to be able to test changes without committing or
pushing changes to Physis. But since we depend on other crates that define
their dependency on Physis you need to do this a certain way.

Add this line to the bottom of your `Cargo.toml`:

```toml
[patch."https://github.com/redstrate/Physis"]
physis = { path = "/path/to/your/Physis" }
```

This will ensure all of the dependencies target your local Physis checkout and you don't end up with multiple conflicting library versions.
