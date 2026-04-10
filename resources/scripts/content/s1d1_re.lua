-- Sastasha

EOBJ_BLOODY_MEMO_BLUE = 2000212
EOBJ_BLOODY_MEMO_RED = 2001548
EOBJ_BLOODY_MEMO_GREEN = 2001549
EOBJ_INCONSPICUOUS_SWITCH = 2000216
EOBJ_BLUE_CORAL_FORMATION = 2000213
EOBJ_RED_CORAL_FORMATION = 2000214
EOBJ_GREEN_CORAL_FORMATION = 2000215
EOBJ_HIDDEN_DOOR = 2000217
EOBJ_NEXT_DOOR1 = 2001506 -- TODO: what is a better name for this?
EOBJ_RAMBADE_DOOR1 = 2000225
EOBJ_CAPTAINS_QUARTERS_DOOR = 2000227
EOBJ_WAVERIDER_GATE = 2000231
EOBJ_THE_HOLE_DOOR = 2000232
EOBJ_CAPTAINS_QUARTERS_KEY = 2000250
EOBJ_WAVERIDER_GATE_KEY = 2000255
EOBJ_KEY_TO_THE_HOLE = 2000256
EOBJ_NEXT_DOOR2 = 2001539
EOBJ_RAMBADE_DOOR2 = 2000236
EOBJ_SHORTCUT = 2000700
EOBJ_EXIT = 2000139

EOBJ_CORAL_IDS = {
    EOBJ_BLUE_CORAL_FORMATION,
    EOBJ_RED_CORAL_FORMATION,
    EOBJ_GREEN_CORAL_FORMATION
}

-- Boss rooms
PLACE_CATTERY = 662
EOBJ_CATTERY_BOSS_WALL = 2001504
EOBJ_CATTERY_BOSS_LINE = 2001505

PLACE_FIRST_RAMBADE = 663
EOBJ_FIRST_RAMBADE_BOSS_WALL = 2001506
EOBJ_FIRST_RAMBADE_BOSS_LINE = 2001507

PLACE_SECOND_RAMBADE = 670
EOBJ_SECOND_RAMBADE_BOSS_WALL = 2001539
EOBJ_SECOND_RAMBADE_BOSS_LINE = 2001540

EVENT_RANGE_BOSS = 4069552

-- Sequence 0
BNPC_GIANT_CLAM1 = 3637470
BNPC_GIANT_CLAM2 = 3637472
BNPC_GIANT_CLAM3 = 3637475
BNPC_GIANT_CLAM4 = 3637473
BNPC_GIANT_CLAM5 = 3637474
BNPC_GIANT_CLAM6 = 3637476

BNPC_RED_CORAL = 4217967
BNPC_BLUE_CORAL = 4217968
BNPC_GREEN_CORAL = 0 -- TODO: figure out layout id
BNPC_CHOPPER = 4035011

-- Sequence 2
BNPC_REAVER1 = 3981887
BNPC_REAVER2 = 3981888
BNPC_CAPTAIN1 = 3988325

-- Sequence 3
BNPC_KEY_HOLDER_REAVER = 3981878
BNPC_CAPTAINS_QUARTERS_REAVER = 3282344

-- Sequence 4
BNPC_REAVER3 = 3978797
BNPC_REAVER4 = 3988324
BNPC_CAPTAIN2 = 4035056

EVENT_ACTION_INTERACT = 24

LOG_MESSAGE_SEQ0 = 2034 -- You hear something move in the distance

EFFECT_POSION = 18

SEQ0 = 0 -- Activate the coral trigger
SEQ1 = 1 -- Open the hidden door
SEQ2 = 2 -- Discover the pirate captain
SEQ3 = 4 -- Obtain the Waverider Gate key
SEQ4 = 8 -- Defeat final boss

-- Randomized coral color
local coral_color
-- Whether the party has the key to The Hole
local has_hole
-- Whether the party has the key to Captain's Quarters
local has_captains_quarters
-- Whether the party has the key to Waverider Gate
local has_waverider_gate
-- Whether the Chopper boss was defeated
local chopper_defeated

function onSetup(director)
    coral_color = math.random(0, 2)

    director:spawn_treasure(98)

    beginSequence0(director)
end

function setSequence(director, sequence)
    director:set_data(0, sequence)
end

function onGimmickAccessor(director, actor_id, id, params)
    -- -1 = will not touch the coral
    -- 0 = will touch the coral
    if has_value(EOBJ_CORAL_IDS, id) and params[1] == 0 then
        director:event_action(EVENT_ACTION_INTERACT, actor_id, id)
        return
    elseif id == EOBJ_INCONSPICUOUS_SWITCH then
        if not chopper_defeated then
            -- Set battle music
            director:set_bgm(37)
            director:spawn_boss(BNPC_CHOPPER, EOBJ_CATTERY_BOSS_WALL, EOBJ_CATTERY_BOSS_LINE, PLACE_CATTERY)
        else
            director:event_action(EVENT_ACTION_INTERACT, actor_id, id)
            return
        end
    elseif id == EOBJ_KEY_TO_THE_HOLE or id == EOBJ_CAPTAINS_QUARTERS_KEY or id == EOBJ_WAVERIDER_GATE_KEY then
        director:event_action(EVENT_ACTION_INTERACT, actor_id, id)
        return
    elseif id == EOBJ_THE_HOLE_DOOR then
        if has_hole_key then
            -- TODO: what happens if they don't have access to The Hole?
            director:event_action(EVENT_ACTION_INTERACT, actor_id, id)
            return
        end
    elseif id == EOBJ_CAPTAINS_QUARTERS_DOOR then
        if has_captains_quarters then
            director:event_action(EVENT_ACTION_INTERACT, actor_id, id)
            return
        end
    elseif id == EOBJ_WAVERIDER_GATE then
        if has_waverider_gate then
            director:event_action(EVENT_ACTION_INTERACT, actor_id, id)
            return
        end
    elseif id == EOBJ_EXIT then
        director:abandon_duty(actor_id)
    end

    director:finish_gimmick(actor_id)
end

function onGimmickRect(director, target)
    if target == EVENT_RANGE_BOSS then
        print("TODO cutscene")
    end
end

function onEventActionCast(director, actor_id, target)
    -- Finish up and hide the event object
    director:finish_gimmick(actor_id)

    if has_value(EOBJ_CORAL_IDS, target) then
        director:hide_eobj(target)

        local coral_gimmick_id = EOBJ_CORAL_IDS[coral_color + 1]
        if target == coral_gimmick_id then
            beginSequence1(director)
        else
            spawnCoralEnemy(director, actor_id, target)
        end
    elseif target == EOBJ_INCONSPICUOUS_SWITCH then
        beginSequence2(director)
    elseif target == EOBJ_KEY_TO_THE_HOLE then
        has_hole_key = true

        director:hide_eobj(target)
    elseif target == EOBJ_WAVERIDER_GATE_KEY then
        has_waverider_gate = true

        director:hide_eobj(target)

        beginSequence4(director)
    elseif target == EOBJ_CAPTAINS_QUARTERS_KEY then
        has_captains_quarters = true

        director:hide_eobj(target)
    elseif target == EOBJ_CAPTAINS_QUARTERS_DOOR then
        -- TODO: when does this guy spawn? when you open the door?
        director:spawn_bnpc(BNPC_CAPTAINS_QUARTERS_REAVER)

        director:hide_eobj(target)
    elseif target == EOBJ_THE_HOLE_DOOR or target == EOBJ_WAVERIDER_GATE then
        director:hide_eobj(target)
    end
end

function onActorDeath(director, bnpc_id, position)
    if bnpc_id == BNPC_CHOPPER then
        director:set_bgm(0) -- Reset music
        chopper_defeated = true
    elseif bnpc_id == BNPC_CAPTAIN1 then
        director:set_bgm(0) -- Reset music
        beginSequence3(director)
    elseif bnpc_id == BNPC_KEY_HOLDER_REAVER then
        director:spawn_eobj(EOBJ_CAPTAINS_QUARTERS_KEY, { x = position.x, y = position.y, z = position.z })
    elseif bnpc_id == BNPC_CAPTAINS_QUARTERS_REAVER then
        director:spawn_eobj(EOBJ_WAVERIDER_GATE_KEY, { x = position.x, y = position.y, z = position.z })
    elseif bnpc_id == BNPC_CAPTAIN2 then
        director:set_bgm(0) -- Reset music
        director:hide_eobj(EOBJ_RAMBADE_DOOR2)
    end
end

-- Spawns an enemy for getting the coral selection wrong, and douse the player in posion.
function spawnCoralEnemy(director, actor_id, target)
    -- TODO: Show message "you were doused with posion"

    director:gain_effect(actor_id, EFFECT_POSION, 0, 120.0)

    if target == EOBJ_BLUE_CORAL_FORMATION then
        director:spawn_bnpc(BNPC_BLUE_CORAL)
    elseif target == EOBJ_RED_CORAL_FORMATION then
        director:spawn_bnpc(BNPC_RED_CORAL)
    elseif target == EOBJ_GREEN_CORAL_FORMATION then
        director:spawn_bnpc(BNPC_GREEN_CORAL)
    end
end

function beginSequence0(director)
    setSequence(director, SEQ0)

    director:hide_eobj(EOBJ_SHORTCUT)

    hideBloodyMemos(director)
    director:hide_eobj(EOBJ_INCONSPICUOUS_SWITCH)

    -- spawn them clams
    director:spawn_bnpc(BNPC_GIANT_CLAM1)
    director:spawn_bnpc(BNPC_GIANT_CLAM2)
    director:spawn_bnpc(BNPC_GIANT_CLAM3)
    director:spawn_bnpc(BNPC_GIANT_CLAM4)
    director:spawn_bnpc(BNPC_GIANT_CLAM5)
    director:spawn_bnpc(BNPC_GIANT_CLAM6)
end

function hideBloodyMemos(director)
    if coral_color ~= 0 then
        director:hide_eobj(EOBJ_BLOODY_MEMO_BLUE)
    end
    if coral_color ~= 1 then
        director:hide_eobj(EOBJ_BLOODY_MEMO_RED)
    end
    if coral_color ~= 2 then
        director:hide_eobj(EOBJ_BLOODY_MEMO_GREEN)
    end
end

function beginSequence1(director)
    setSequence(director, SEQ1)

    -- Show log message to help indicate to the player
    director:log_message(LOG_MESSAGE_SEQ0, {})

    -- Hide and deactivate all coral
    director:hide_eobj(EOBJ_BLUE_CORAL_FORMATION)
    director:hide_eobj(EOBJ_RED_CORAL_FORMATION)
    director:hide_eobj(EOBJ_GREEN_CORAL_FORMATION)

    director:show_eobj(EOBJ_INCONSPICUOUS_SWITCH)
end

function beginSequence2(director)
    setSequence(director, SEQ1 | SEQ2) -- TODO: lol this looks awkward

    director:hide_eobj(EOBJ_INCONSPICUOUS_SWITCH)
    director:hide_eobj(EOBJ_HIDDEN_DOOR)

    director:hide_eobj(EOBJ_NEXT_DOOR1)

    -- Spawn captain and his goons
    director:spawn_bnpc(BNPC_REAVER1)
    director:spawn_bnpc(BNPC_REAVER2)

    -- Set battle music
    director:set_bgm(37)
    director:spawn_boss(BNPC_CAPTAIN1, EOBJ_FIRST_RAMBADE_BOSS_WALL, EOBJ_FIRST_RAMBADE_BOSS_LINE, PLACE_FIRST_RAMBADE)
end

function beginSequence3(director)
    setSequence(director, SEQ1 | SEQ2 | SEQ3)

    director:hide_eobj(EOBJ_RAMBADE_DOOR1)

    -- TODO: is this the same BNPC every playthrough?
    director:spawn_bnpc(BNPC_KEY_HOLDER_REAVER)
end

function beginSequence4(director)
    setSequence(director, SEQ1 | SEQ2 | SEQ3 | SEQ4)

    director:hide_eobj(EOBJ_NEXT_DOOR2)

    -- Spawn captain and his goons *AGAIN*
    director:spawn_bnpc(BNPC_REAVER3)
    director:spawn_bnpc(BNPC_REAVER4)

    -- TODO: spawn them dogs

    -- Set battle music
    director:set_bgm(37)
    director:spawn_boss(BNPC_CAPTAIN2, EOBJ_SECOND_RAMBADE_BOSS_WALL, EOBJ_SECOND_RAMBADE_BOSS_LINE, PLACE_SECOND_RAMBADE)
end
