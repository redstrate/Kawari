-- Limsa Lominsa opening
-- NOTE: These openings are so similar, ensure changes are synced between all of them!

SCENE_OPENING         = 00000 -- The actual opening cutscene
SCENE_CONTROLS_HOWTO  = 00001 -- The starting text asking you which controls you want to use.
SCENE_BOUNDS_CHECK1   = 00010 -- "Where are you going?" message
SCENE_BOUNDS_CHECK2   = 00020 -- Another "where are you going?" message
SCENE_RESETUP_BOUNDS  = 00040 -- Sets up the bounds based on the current sequence

function onEnterTerritory(player)
    local sequence = determineSequence(player, NCUT_LIGHT_ALL)
    if sequence == OPENING_SEQ_0 then
        player:play_scene(SCENE_OPENING, INITIAL_CUTSCENE_FLAGS, {0})
    else
        player:play_scene(SCENE_RESETUP_BOUNDS, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {sequence})
    end
end

function onReturn(scene, results, player)
    -- Move into the controls text after initial cutscene
    if scene == 0 then
        player:play_scene(SCENE_CONTROLS_HOWTO, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
        return
    end

    player:finish_event()
end

function onEnterTrigger(player, arg)
    -- Play the "where are you going?!" text when entering any trigger
    if arg == ERANGE_SEQ_1_CLOSED_1 then
        player:play_scene(SCENE_BOUNDS_CHECK2, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
    else
        -- Deciding the different messages and NPCs are actually handled client-side!
        player:play_scene(SCENE_BOUNDS_CHECK1, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {arg})
    end
end
