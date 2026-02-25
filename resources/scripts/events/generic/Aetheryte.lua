-- generic aetheryte, use this for all of the big aetherytes

-- Scenes
SCENE_SHOW_MENU             = 00000 -- display main menu ("Welcome to <location>.")
SCENE_00001                 = 00001 -- softlocks, unknown purpose
SCENE_00002                 = 00002 -- aethernet menu (used only by aethernet shards? scene 00 for big aetherytes display the aethernet submenu without needing an additional play_scene)
SCENE_HAVE_AETHERNET_ACCESS = 00003 -- "you have aethernet access" message and vfx
SCENE_00100                 = 00100 -- "According to the message engraved in the base, special permission is required to use this aetheryte." (Eulmore-specific)
SCENE_00200                 = 00200 -- "The aetheryte has ceased functioning." (Eulmore-specific)

function aetheryteId()
    return EVENT_ID & 0xFFFF
end

function onTalk(target, player)
    if not player:has_aetheryte(aetheryteId()) then
        -- TODO: play attunement animation
        player:unlock_aetheryte(1, aetheryteId())
    end

    player:play_scene(SCENE_SHOW_MENU, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
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
            player:set_homepoint(aetheryteId())
        elseif menu_option == AETHERNET_SUBMENU then
            if decision ~= AETHERNET_SUBMENU_CANCEL then
                player:finish_event() -- Need to finish the event here, because warping does not return to this callback (the game will crash or softlock otherwise)
                player:warp_aetheryte(decision)
                return
            end
        --[[elseif menu_option == REGISTER_FAVORITE_DSTN then
            -- TODO: Favorite Destination logic
        else -- REGISTER_SECURITY_TOKEN_DSTN
            -- TODO: Security Token Free Destination logic ]]
        elseif menu_option == ACCESS_RESIDENTAL_AREA then
            -- determine the target residental area (they don't maintain a mapping, so we do)
            local territory_id = player.zone.id
            local housing_id
            if territory_id == TERRITORY_S1T2 then
                housing_id = TERRITORY_S1H1
            elseif territory_id == TERRITORY_F1T1 then
                housing_id = TERRITORY_F1H1
            elseif territory_id == TERRITORY_W1T1 then
                housing_id = TERRITORY_W1H1
            elseif territory_id == TERRITORY_E3T1 then
                housing_id = TERRITORY_E1H1
            elseif territory_id == TERRITORY_R2T1 then
                housing_id = TERRITORY_R1H1
            else
                print("Unknown housing territory for: "..territory_id)
            end

            -- open the housing menu
            player:start_event(1310721, EVENT_TYPE_NEST, 0)
            player:play_scene(0, HIDE_HOTBAR | NO_DEFAULT_CAMERA, {housing_id})
            return
        end
    end

    player:finish_event()
end
