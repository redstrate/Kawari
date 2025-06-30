-- TODO: actually implement this menu

-- Scene 00000: NPC greeting (usually an animation, sometimes text too?)
-- Scene 00010: Displays shop interface
-- Scene 00255: Unknown, but this was also observed when capturing gil shop transaction packets. When used standalone it softlocks.

function onTalk(target, player)
    --[[ Params observed:
        Gil shops: [0, 1]
        Non- shops: [1, 0]
        MGP shops: [1, 100]
        It's unclear what these mean since shops seem to open fine without these.
    ]]
    player:play_scene(target, EVENT_ID, 00000, 8192, {0})
end

function onReturn(scene, results, player)
    if scene == 0 then
        --[[ Retail sends 221 zeroes as u32s as the params to the shop cutscene, but it opens fine with a single zero u32.
            Perhaps they are leftovers from earlier expansions? According to Sapphire, the params used to be significantly more complex.
            Historically, it also seems cutscene 00040 was used instead of 00010 as it is now.
        ]]
        player:play_scene(player.id, EVENT_ID, 00010, 1 | 0x2000, {0})
    elseif scene == 10 then
        player:finish_event(EVENT_ID)
    end
end
