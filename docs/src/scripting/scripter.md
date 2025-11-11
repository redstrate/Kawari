# Using Scripter

The client has it's own Lua state that it uses to play cutscenes, manage event NPCs and more. We can peek into this opaque system with tools like [HaselDebug](https://github.com/Haselnussbomber/HaselDebug/) or [Scripter](https://codeberg.org/redstrate/Scripter/).

This guide will focus on Scripter, but some of this can be done with HaselDebug as well.

# Inspector/Globals

These tabs shows you the current Lua global table. Inside is things like defined globals, and functions which you can use to run custom Lua code.

# Scripts

This tab shows you which entities are associated with a client-side Lua script. How scripts are organized in the client files are not super obvious, so if you need to reverse engineer an entity it's easier to walk up to it with Scripter enabled.

The game ships pre-compiled Lua scripts by default, so they're in bytecode form - not as text files. However there's many, many decompilers online. [Novus](https://github.com/redstrate/Novus) has a built-in Lua decompiler in its Data Explorer program.

# Code

This tab allows you to run custom Lua code, assuming you know how to. This is an undocumented process right now.
