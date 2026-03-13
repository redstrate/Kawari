# Infrastructure

In the event of a project closure, forking or other event this document should be used to keep track of the state of our infrastructure and access.

## Code repository

The project source code is hosted on GitHub, and currently the only user with write privileges is [@redstrate](https://github.com/redstrate).

We appreciate any mirrors of the source code, however the official one is at [code.ryne.moe](https://code.ryne.moe/redstrate/kawari) in the event the GitHub one is inaccessible. This is only meant as a read-only mirror and is not set up for contributions.

## Continuous Integration

The project uses GitHub Actions for CI, and only uses the hosted GitHub runners. No self-hosted runners are used, so forked projects should have little trouble replicating our CI setup.

Our workflow does involve uploading artifacts to xiv.zone to avoid GitHub lock-in, however this can be omitted and the rest of the workflow should function fine. These artifacts include binaries as well as project documentation, which are easily reproducible locally.

## Chat

The Matrix chat is maintained by @redstrate, who is also the room's creator. The room is currently low traffic enough that moderation is done solely by @redstrate. Matrix is decentralized so even if the domain pyra.sh is offline, the room will still function for everyone else.

The room is invite-only, with access gated behind a simple bot @bot:pyra.sh. The reason is two-fold:
* It should keep out most dumb spammers who enter public rooms.
* If my server shuts down for whatever reason the room is "locked" from the outside. It would still be unmoderated, but it's at least a good start.

The source code for the bot is located here.
