# Updating

If you're updating Kawari to a new game version, there's a mix of automated and manual steps. Some of these can only be done if you have privileged access to the repository or authorized SSH  keys. The purpose of this page is to ensure all of these steps are at least documented for future maintainers.

## Determine the target game version

The first step should be determining which version you're updating to. **For technical reasons, you need to go patch-by-patch.** That means if there was two hotfixes released and you skipped one, you still need to follow this guide.

The first version you'll need is the patch version (e.g. "2026.06.18.0000.0000") and that can be checked in any launcher after updating. The second version you'll need is the "patch name", such as 7.51h2.

## Run automated updater

A bunch of previous, tedious work has been automated into a tool called KawariUpdater. You can find said tool [here](https://codeberg.org/redstrate/KawariUpdater). Plugging in your client path and the versions mentioned above, you should have a working (or almost working) Kawari for that version!

## Update Icarus (if necessary)

Sometimes the client will update and break existing Excel schemas, requiring [EXDSchema](https://github.com/XIVDev/EXDSchema) to release a new version. If this happens, an authorized contributor needs to run [EXDGen](https://codeberg.org/redstrate/EXDGen), push that branch to [Icarus](https://github.com/redstrate/Icarus) and then finally point Kawari to that new version.

Code changes may be required when porting to a new version of Icarus.

## Check functionality

If you have the time, run through the [in-game checklist](update_ingame_checklist.md) and confirm what does and doesn't work. It's encouraged to then open a new patch regression issue to keep track of what broke.

## Push changes

Assuming [you are allowed to do so](acceptable_usage.md) you may open a pull request to the repository. Even for patch updates, please run through the [contributing checklist](contributing.md#checklist).

## Create tag

Once the patch update is merged, an authorized contributor will create a tag of the old version on the previous commit. The tag name should be the patch version (e.g. "2026.06.18.0000.0000"). This serves as a useful historical marker.

## Update PacketAnalyzer

We host an instance of [Packet Analyzer](https://codeberg.org/redstrate/PacketAnalyzer) at [analyze.xiv.zone](https://analyze.xiv.zone). An authorized contributor must run `cargo update` in the repository, change the `VERSION` variable in `scripts/copy-to-server.sh` and then run `scripts/build-for-web.sh` and `scripts/copy-to-server.sh`. Then in the server's filesystem, modify `versions.json` to add the new patch version (e.g. "2026.06.18.0000.0000").

After all of the required changes are pushed, create a tag similar to the Kawari step above.
