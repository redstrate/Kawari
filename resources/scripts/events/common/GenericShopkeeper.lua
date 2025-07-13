-- TODO: actually implement this menu

-- Scene 00000: NPC greeting (usually an animation, sometimes text too?)
-- Scene 00010: Displays shop interface
-- Scene 00255: Unknown, but this was also observed when capturing gil shop transaction packets. When used standalone it softlocks.

SCENE_GREETING  = 00000
SCENE_SHOW_SHOP = 00010
SCENE_SHOP_END  = 00255
function onTalk(target, player)
    --[[ Params observed for SCENE_GREETING:
        Gil shops: [0, 1]
        Non- shops: [1, 0]
        MGP shops: [1, 100]
        It's unclear what these mean since shops seem to open fine without these.
    ]]
    player:play_scene(target, EVENT_ID, SCENE_GREETING, 8192, {0})
end

function onReturn(scene, results, player)
        --[[ Retail sends 221 zeroes as u32s as the params to the shop cutscene, but it opens fine with a single zero u32.
            Perhaps they are leftovers from earlier expansions? According to Sapphire, the params used to be significantly more complex.
            Historically, it also seems cutscene 00040 was used instead of 00010 as it is now.
            When the shop scene finishes and returns control to the server, the server will then have the client play scene 255 with no params.
        ]]
    if scene == SCENE_GREETING then
        params = {}
        for i=1,221 do
            params[i] = 0
        end
        player:play_scene(player.id, EVENT_ID, SCENE_SHOW_SHOP, 1 | 0x2000, params)
    elseif scene == SCENE_SHOW_SHOP then
        player:play_scene(player.id, EVENT_ID, SCENE_SHOP_END, 1 | 0x2000, {})
    else
        player:finish_event(EVENT_ID)
    end
end
