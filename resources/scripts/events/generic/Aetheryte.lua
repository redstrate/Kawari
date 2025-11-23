-- generic aetheryte, use this for all of the big aetherytes

--- scene 00000 - display main menu ("Welcome to <location>.")
--- scene 00001 - softlocks, unknown purpose
--- scene 00002 - aethernet menu (used only by aethernet shards? scene 00 for big aetherytes display the aethernet submenu without needing an additional play_scene)
--- scene 00003 - "you have aethernet access" message and vfx
--- scene 00100 - "According to the message engraved in the base, special permission is required to use this aetheryte." (Eulmore-specific)
--- scene 00200 - "The aetheryte has ceased functioning." (Eulmore-specific)

SCENE_SHOW_MENU = 00000
SCENE_HAVE_AETHERNET_ACCESS = 00003

function onTalk(target, player)
    player:play_scene(target, EVENT_ID, SCENE_SHOW_MENU, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    local AETHERNET_SUBMENU = 4
    local AETHERNET_SUBMENU_CANCEL = 0
    local SET_HOME_POINT = 1
    local HOME_PNT_YES = 1
    local HOME_PNT_NO = 0
    -- local REGISTER_FAVORITE_DSTN = ??? -- Unable to obtain right now, seems to return 0 regardless
    -- local REGISTER_SECURITY_TOKEN_DSTN = ??? -- Unable to obtain right now, seems to return 0 regardless
    local ACCESS_RESIDENTAL_AREA = 5

    local resultc = #results -- TODO: Do we need to check this still? Can the favorite/security menus return more than 2, once they work?

    local menu_option = results[1]
    local decision = results[2]

    if scene == SCENE_SHOW_MENU then -- main aetheryte prompt scene
        if menu_option == SET_HOME_POINT then
            -- TODO: logic for setting home point
            --[[ if decision == HOME_PNT_YES then
            else
            end ]]
        elseif menu_option == AETHERNET_SUBMENU then
            if decision ~= AETHERNET_SUBMENU_CANCEL then
                player:finish_event(EVENT_ID) -- Need to finish the event here, because warping does not return to this callback (the game will crash or softlock otherwise)
                player:warp_aetheryte(decision)
                return
            end
        --[[elseif menu_option == REGISTER_FAVORITE_DSTN then
            -- TODO: Favorite Destination logic
        else -- REGISTER_SECURITY_TOKEN_DSTN
            -- TODO: Security Token Free Destination logic ]]
        elseif menu_option == ACCESS_RESIDENTAL_AREA then
            -- open the housing menu
            player:start_event(player.id, 1310721, EVENT_TYPE_NEST, 0)
            player:play_scene(player.id, 1310721, 0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {340})
            return
        end
    end

    player:finish_event(EVENT_ID)
end
