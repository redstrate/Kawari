# Dissecting Packets

At this point, you should have one or more `.cfcap` captures from your retail game session. However they aren't readable out-of-the-box, and we'll need Chronofoil-aware tools to view its contents.

We recommend trying our own tool called Packet Analyzer first. It's not only available online, but it re-uses the core parts of Kawari for packet parsing. Visit [analyze.xiv.zone](https://analyze.xiv.zone), open your `.cfcap` and its contents will show up for your perusal.

Each capture is simply a collection of "frames" (bundles of packets, sent together) over several "connections" (e.g. the Zone connection) and inside are the packets themselves. The vocabulary used by the community isn't standardized, so I apologize for any confusion.

As we mentioned before, Packet Analyzer can parse supported packets just like Kawari does. For example, if you take a look at a chat message you can see its contents:

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

This is reuses the same packet definitions from Kawari, so when you [contribute](../contributing.md) to Kawari you're also making packets easier to read for everyone. On the other hand, the parsed view shouldn't be fully trusted since there could be mistakes in our parsing.

Now that you know how to dissect packets, we suggest [reading potentially useful tips on reverse engineering them](tips.md).
