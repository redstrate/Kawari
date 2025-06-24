-- generic aetheryte, use this for all of the big aetherytes

--- scene 02 - aetheryte menu
--- scene 03 - "you have aethernet access" message and vfx
--- scene 100 - "According to the message engraved in the base, special permission is required to use this aetheryte." (Eulmore-specific)
--- scene 200 - "The aetheryte has ceased functioning." (Eulmore-specific)

function onTalk(target, player)
    --- param has to be 1 for the menu to even show up
    player:play_scene(target, EVENT_ID, 00000, 8192, 1)
end

function onReturn(scene, results, player)
    --- results [3] is 1 if the player requested to set it as their home point
    player:finish_event(EVENT_ID)
end
