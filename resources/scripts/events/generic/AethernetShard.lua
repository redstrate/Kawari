-- generic aetheryte, use this for all of the aethernet shards

--- scene 00000 - does nothing
--- scene 00001 - does nothing
--- scene 00002 - aetheryte menu
--- scene 00003 - "you have aethernet access" message and vfx
--- scene 00100 - "According to the message engraved in the base, special permission is required to use this aetheryte." (Eulmore-specific)
--- scene 00200 - "The aetheryte has ceased functioning." (Eulmore-specific)

SCENE_SHOW_MENU = 00002
SCENE_HAVE_AETHERNET_ACCESS = 00003

function aetheryteId()
    return EVENT_ID & 0xFFFF
end

function onTalk(target, player)
    if not player:has_aetheryte(aetheryteId()) then
        -- TODO: play attunement animation
        player:unlock_aetheryte(1, aetheryteId())
    end

    player:play_scene(target, SCENE_SHOW_MENU, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    local AETHERNET_MENU_CANCEL = 0
    local destination = results[1]

    if scene == SCENE_SHOW_MENU then
        if destination ~= AETHERNET_MENU_CANCEL then
            player:finish_event() -- Need to finish the event here, because warping does not return to this callback (the game will crash or softlock otherwise)
            player:warp_aetheryte(destination)
            return
        end
    --elseif scene == HAVE_AETHERNET_ACCESS then
        -- TODO: attunement logic
    end

    player:finish_event()
end
