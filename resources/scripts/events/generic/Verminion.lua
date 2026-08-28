-- Lord of Verminion tables in the Gold Saucer

function onTalk(target, player)
    local lastMinionStage = 24 -- Max out for now
    player:play_scene(00000, HIDE_HOTBAR, {411, lastMinionStage})
end

function onReturn(scene, results, player)
    if scene == 0 and #results == 2 then
        -- Asked for a tutorial challenge
        local minionStage = results[2] -- Index into the MinionStage sheet?? (technically, not its not super useful)
        local goldSaucerContentId = minionStage + 58 -- Turn into a row into GoldSaucerContent
        player:register_for_content(GAME_DATA:lookup_gold_saucer_content(goldSaucerContentId))
    end
    player:finish_event()
end

function onYield(scene, id, results, player)
    -- TODO: used for Tournament information...
end
