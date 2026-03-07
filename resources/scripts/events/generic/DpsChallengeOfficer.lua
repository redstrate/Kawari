-- Generic DPS challenge officers (for Stone, Sea, Sky)

-- Scene
SCENE_00000 = 00000 -- Open menui

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    -- 0 when exiting the menu normally, otherwise returns an index into the DpsChallenge Excel sheet.
    if results[1] > 0 then
        -- TODO: implement warping to the location
    end
    player:finish_event()
end
