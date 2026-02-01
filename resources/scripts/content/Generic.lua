function onEnterTerritory(player)
    -- TODO: figure out scene params
    player:play_scene(player.id, 1, NO_DEFAULT_CAMERA | CONDITION_CUTSCENE | HIDE_HOTBAR | SILENT_ENTER_TERRI_ENV | SILENT_ENTER_TERRI_BGM | SILENT_ENTER_TERRI_SE | DISABLE_STEALTH | DISABLE_CANCEL_EMOTE | INVIS_AOE | UNK1, {
        0, -- BGM, according to sapphire?
        0,
        0,
        5,
        14400,
        0,
        0,
        0,
        0,
        0,
        player.content.duration,
        player.content.settings,
    })
end

function onYield(scene, results, player)
    player:commence_duty(EVENT_ID)
    player:finish_event()
end
