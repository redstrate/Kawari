-- Delivery Moogles in various towns and Letter Boxes which provide the same function in housing wards.

-- Scenes
SCENE_MAIL_MENU = 00000 -- Displays the Moogle Delivery Service window.

CONDITION = CONDITION_OCCUPIED_IN_EVENT

function onTalk(target, player)
    player:play_scene(SCENE_MAIL_MENU, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    player:finish_event()
    player:send_mailbox_status()
end
