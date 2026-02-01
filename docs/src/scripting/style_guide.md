# Events

Events should follow this general structure:

```lua
-- Write the (English) name here, or "Unknown object/NPC" if you don't know/remember

-- TODO: any outstanding that should be fixed eventually

-- Scenes
SCENE_00000 = 00000 -- This does something interesting
SCENE_00001 = 00001 -- Likewise
... and so on ...

EFFECT_DURATION = 1800.0 -- Any globals should be here

function onTalk(target, player)
    ...
end

function onYield(scene, results, player)
    ...
end
```

If a scene has a known and clear purpose (such as "waking up from a bed") then write it as such. Otherwise give it a number, and ensure these numbers are always padded to five zeroes. Scenes should always be documented to the best of your ability.
