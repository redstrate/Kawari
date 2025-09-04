# Packet Tips

Here are some tips about packet capture and reverse engineering.

## Capturing also works on Kawari

Chronofoil - along with other tools - work with Kawari as well (and other servers.) This can be used to debug server-specific issues, and has been proven to be useful in fixing Kawari-specific issues.

## Packets are always fixed size

One oddity that trips people up is how FFXIV handles "dynamically sized" packets. One example is the game's Lua event system that supports sending an array of integers. Someone new to packet RE may erroneously think there's only one kind of packet, there's a size field and that tells us how big the following integer array is.

In reality, _all_ packets in the game are fixed-size. Yes, this means for dynamically sized packets there is several versions of said packet - each meant for the max size of their array. In our "sending an array of integers" example there is:

* `EventScene2` (holding 2 integers max)
* `EventScene4` (holding 4 integers max)
* `EventScene8` (holding 8 integers max)
* And so on, you get the idea!

## Where to find existing opcodes/packets

There are a multitude of other projects that can be used for reference, inspiration or documentation:

* [Sapphire Server](https://github.com/SapphireServer/Sapphire) - which has branches for both 3.x and 5.x eras.
* [Maelstrom](https://github.com/Rawaho/Maelstrom) which is from the 4.x era.
* [iolite](https://github.com/0xbbadbeef/iolite) which targets the modern era.
* [FFXIVOpcodes](https://github.com/karashiiro/FFXIVOpcodes/) which has a limited selection of opcodes.

## Blocking opcodes in retail

Something we have needed from time to time is a way to truly understand some obscure packet seen in retail. One way is to "block" the client from recieving it and seeing what happens. This might not seem super useful at first, but it's been used to fix problems like long loading times - by figuring out which packets are truly needed during zone loading.

**NOTE:** This process could potentially be interpreted as cheating, messing with retail network operation is inherently risky. Do not try this on an account you care about!

You can use the [Firewall plugin](https://codeberg.org/redstrate/Firewall) for this task. Due to the dangerous nature of the plugin, you will have to figure out how to build it yourself.
