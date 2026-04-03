-- "Entrance to Additional Chambers", found in all free company houses (and for now, on Kawari, found in *all* cottage, house and mansion interiors)

-- TODO: Make this menu actually work

--Scenes
SCENE_MENU = 00000 -- Displays menu to enter others' private chambers, one's own private chamber, and the Company Workshop.

function onTalk(target, player)
    player:play_scene(SCENE_MENU, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
end

function onYield(scene, yield_id, results, player)
    player:send_message("Housing is currently unimplemented, so this menu is non-functional.")
    player:resume_event(SCENE_MENU, yield_id, {0})
end
