# Contributing and working on Kawari

Here are various helpful resources and tips when working on Kawari.

## Recommended Dalamud plugins

Here are some Dalamud plugins that will make your life easier:

* [AllowLoginFail](https://codeberg.org/redstrate/AllowLoginFail) to stop the game from rage quitting after hitting lobby errors.
* [Scripter](https://codeberg.org/redstrate/Scripter) for inspecting the client's Lua state.

## Updating to new patches

Updating Kawari can be quite involved, and so we developed a tool to automate the process. You can find said tool [here](https://codeberg.org/redstrate/KawariUpdater).

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
