-- Generic handler for GimmickAccessor events

-- TODO: figure out how shortcuts work

function onTalk(target, player)
    player:play_scene(1, HIDE_HOTBAR, {})
end

-- Yielding/Finishing is handling on the Rust side
