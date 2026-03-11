# Scripting Events

We currently support scripting CustomTalk and Quest events with Lua.

## Functions

You can define the following functions to handle various callbacks:

### `onTalk(target, player)`

This is called when a player begins talking with this actor.

* `target` is the ID of the actor being spoken to.
* `player` is the `LuaPlayer` which represents the player.

### `onReturn(scene, results, player)`

This is called when a scene returns.

* `scene` is the scene number.
* `results` is an array of results returned by the client.
* `player` is the `LuaPlayer` which represents the player.

### `onYield(scene, id, results, player)`

This is called when a scene yields execution.

* `scene` is the scene number.
* `id` is the yield ID. This has no meaningful value onto itself.
* `results` is an array of results returned by the client.
* `player` is the `LuaPlayer` which represents the player.

## `LuaPlayer` methods

Here is a non-exhaustive list of event-related methods on `LuaPlayer`:

### `play_scene(scene, flags, params)`

Starts `scene` given `flags` and a list of `params`.

### `finish_event()`

Forcefully finishes the current event, and either returns control back to the player or the event that nests this one.

### `start_event(id, type, arg)`

Starts another event of `id`, usually paired with `EVENT_TYPE_NEST` as `type` for nesting.

## Debugging

Here's a few things to try if you get stuck:

* Using [Scripter](scripter.md) and other tools to decompile the client's Lua bytecode, which will help understanding the scene flow.
* Paying attention to the server log, which will inform you when scenes return and yield.
* [Capturing packets](../packets/intro.md) to see how the scenes flow in-game.

## Maintainability

Please follow these guidelines to the best of your ability, so we can keep event scripts nice and clean:

* Follow the [style guide](style_guide.md).
* A list of injected variables is printed in the server log when your event starts. Try to use instead of hard-coding globals, as these will be updated automatically by the game.
