-- TODO: actually implement this menu

-- Scene 00000 will just say you can't participate or bug out (in the case of ARR hunt boards), and is also the only scene this actor has

function onTalk(target, player)
    -- In expansion towns, this will display text such as "The Clan Hunt board is covered in bills showing the details of wanted monsters. However, you are unable to accept any of the bills posted at present."
    -- In the ARR capitals it will still open an empty buggy hunt bill list. Presumably they work differently and hide the board entirely when you're unable to participate in The Hunt.
    player:play_scene(target, 00000, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
