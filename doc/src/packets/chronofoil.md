# Setting up Chronofoil

[Project Chronofoil](https://github.com/ProjectChronofoil) is designed to help make capturing network traffic in FFXIV easy, and it comes packaged as a [Dalamud plugin](https://github.com/goatcorp/Dalamud). You can read more information [about the project in their README](https://github.com/ProjectChronofoil/Chronofoil.Plugin?tab=readme-ov-file#chronofoilplugin).

## Installation

Follow the guide [in their README](https://github.com/ProjectChronofoil/Chronofoil.Plugin?tab=readme-ov-file#installation) for more information. Once it is installed, you should be able to bring up the Chronofoil Settings window.

## Usage

Chronofoil automatically begins capturing traffic as soon you connect to a Data Center, and stops when you log out or close the game. You can see existing captures in the Chronofoil Settings window, but they exist as normal files on your filesystem.

**By default, your captures are not uploaded anywhere.** You're not required to do so for our purposes, but it's still highly recommended as it helps archive the game's network protocol.

By default, Chronofoil saves captures to `%APPDATA%\Local\chronofoil\`. (On Linux or macOS, this is of course located relative to your Wine prefix.) 

Completed captures are saved as `.cfcap` files, and you'll need one for the remaining steps. Once you located yours, you can proceed to [viewing their contents](dissection.md).
