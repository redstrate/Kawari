# Windows

This guide covers how to setup Kawari on Windows, which also includes Wine.

## Requirements

* Legally obtained copy of the game that's updated to a supported version
* [Visual C++ Redistributable for Visual Studio 2015](https://www.microsoft.com/en-US/download/details.aspx?id=48145)
* Oodle Network Compression
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). Download the "msvc.zip" file.

## Downloading Kawari

Download the [latest build for Windows from xiv.zone](https://xiv.zone/distrib/kawari/Kawari-Windows.zip) (or pick a specific build from [Github Actions](https://github.com/redstrate/Kawari/actions).)

## Setup

Place the `oodle-network-shared.dll` from the Oodle zip you downloaded next to the Kawari executables.

## Reverse proxy setup

{{#include reverse_proxy.md}}

## Configuration

{{#include configuration.md}}

## Running

Run the server by executing `run.bat`.

## Logging in

{{#include logging_in.md}}
