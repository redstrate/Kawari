--[[ Scene 0 informs the player that a GATE is currently active, where it's taking place, and has the following parameters:
    First is the GATE, second is the location. See scene 2 notes for GATE ids and location ids.
]]

-- Scene 1 informs the player about an ongoing GATE that can't be participated in (Wind, Slice) but can still be observed and likely shares the same parameters as scene 0. No capture has been made yet, though.

--[[ Scene 2 informs the player about when and where the next GATE will take place, and has the following parameters:
    First one is a Unix timestamp

    Second is the GATE:
    1 is Cliffhanger
    2 is Vase Off (a defunct gate)
    3 is Skinchange We Can Believe In (an old defunct gate)
    4 is The Time of My Life (another defunct gate)
    5 is Any Way The Wind Blows
    6 is Leap of Faith
    7 is Air Force One
    8 is Slice is Right
    -Anything not in the range of 1-9 is " " or causes a softlock
    
    Third is the location:
    1 is Wonder Square East
    2 is Event Square
    3 is Round Square
    4 is Cactpot Board (???)
    -Anything not in that range results in " " or a softlock

    The fourth is unknown, possibly flags or a destination id of some sort
]]

function onTalk(target, player)
    -- Currently using placeholders for now: Cliffhanger in Wonder Square East.
    player:play_scene(target, 00000, HIDE_HOTBAR, {1, 1})
end

function onYield(scene, results, player)
    -- first result is 1 if requesting to warp, otherwise 0
    player:finish_event()
end
