# Setting up TemporalStasis.Chronofoil

[TemporalStasis.Chronofoil](https://github.com/NotNite/TemporalStasis.Chronofoil) is meant to make capturing network traffic in FFXIV easy, and is a proxy that sits between your client and the server. This is a program built on top of [TemporalStasis](https://github.com/NotNite/TemporalStasis) that serializes data into the Chronofoil capture format.

## Installation

There are no pre-built binaries at the moment, but simply clone the repository and run `dotnet build`.

## Usage

Run the binary `TemporalStasis.Chronofoil` in the `bin` folder. You will have to specify a path to the Oodle library:

```shell
./TemporalStasis.Chronofoil --oodle-path liboodle-network-shared.so
```

Then launch the game, and connect to the lobby server at `127.0.0.1:44994`. How to accomplish this is left as an exercise for the reader.

Completed captures are saved as `.cfcap` files, and you'll need one for the remaining steps. Once you located yours, you can proceed to [viewing their contents](dissection.md).
