# Dissecting Packets

At this point, you should have one or more `.cfcap` captures from your retail game session. However they aren't readable out-of-the-box, and we'll need Chronofoil-aware tools to dissect/view their contents.

## Packet Analyzer

The first tool in our repertoire is visually easy to understand. It's available at [analyze.xiv.zone](https://analyze.xiv.zone), all you have to do is point to your `.cfcap` and its contents will show up for your perusal.

Each capture is simply a collection of "frames" (bundles of packets, sent together) over several "connections" (e.g. the Zone connection) and inside are the packets themselves. (The vocabulary used by the community isn't standardized, I apologize for any confusion.)

In comparison to other dissection tools, Packet Analyzer can also "parse" packets - where supported. For example, if you take a look at a chat message you can see the string contents:

```rust
Ipc(
    IpcSegment {
        unk1: 20,
        unk2: 0,
        op_code: SendChatMessage,
        option: 4,
        timestamp: 1755210231,
        data: SendChatMessage(
            SendChatMessage {
                actor_id: 277803313,
                pos: Position {
                    x: -10.05261,
                    y: 91.49967,
                    z: -16.038721,
                },
                rotation: -0.38939738,
                channel: Say,
                message: "im in",
            },
        ),
    },
)
```

This is reuses the same packet definitions in Kawari, so when you contribute to Kawari you're also making packets easier to read for everyone. On the other hand, the parsed view can't be fully trusted because there could be mistakes with our parsing.

## cfcap-expand

This is a legacy tool, a pre-cursor to Packet Analyzer but still contains some useful features that hasn't been ported. It's a CLI tool that basically "unzips" your `.cfcap` file. You may want this if you:

* Need the raw IPC data to open in your favorite hex editor.
* Need to check packet sizes against Kawari's definitions, as it has a linter for this.

There is no pre-built version, but if you built Kawari then you already have all of the tools necessary. A guide to using cfcap-expand is [located in it's repository](https://codeberg.org/redstrate/XIVPacketTools#cfcap-expand).

Now that you know how to dissect packets, we suggest [reading potentially useful tips on reverse engineering them](tips.md).
