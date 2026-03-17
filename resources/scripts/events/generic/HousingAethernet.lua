SCENE_SHOW_MENU = 00000 -- Displays the housing aethernet menu

function onTalk(target, player)
    -- Housing ward aethernet shards are always unlocked, no need to do anything with attunement
    player:play_scene(SCENE_SHOW_MENU, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {0})
end

function onReturn(scene, results, player)
    -- If the player cancelled without warping, no results are sent.
    if #results == 1 then
        player:finish_event() -- Need to finish the event here, because warping does not return to this callback (the game will crash or softlock otherwise)
        player:warp_aetheryte(results[1], true)
        return
    end
    player:finish_event()
end
