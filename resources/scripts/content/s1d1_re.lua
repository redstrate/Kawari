-- Satasha

function onEnterTerritory(player)
    -- TODO: figure out scene params
    player:play_scene(player.id, EVENT_ID, 1, NO_DEFAULT_CAMERA | CONDITION_CUTSCENE | HIDE_HOTBAR | SILENT_ENTER_TERRI_ENV | SILENT_ENTER_TERRI_BGM | SILENT_ENTER_TERRI_SE | DISABLE_STEALTH | DISABLE_CANCEL_EMOTE | INVIS_AOE | UNK1, {
        0,
        0,
        0,
        5,
        14400,
        0,
        0,
        0,
        0,
        0,
        5400, -- Duration?
        262152,
        0,
        0,
        0,
        0,
    })
end

function onYield(scene, results, player)
    player:commence_duty(EVENT_ID)
    player:finish_event(EVENT_ID)
end
