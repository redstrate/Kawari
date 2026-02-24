-- Sastasha

EOBJ_BLOODY_MEMO_BLUE = 2000212
EOBJ_BLOODY_MEMO_RED = 2001548
EOBJ_BLOODY_MEMO_GREEN = 2001549
EOBJ_INCONSPICUOUS_SWITCH = 2000216
-- TODO: these are *technically* duplicating the IDs below, and that should probably be fixed
EOBJ_BLUE_CORAL_FORMATION = 2000213
EOBJ_RED_CORAL_FORMATION = 2000214
EOBJ_GREEN_CORAL_FORMATION = 2000215
EOBJ_HIDDEN_DOOR = 2000217
EOBJ_NEXT_DOOR = 2001506 -- TODO: what is a better name for this?
EOBJ_RAMBADE_DOOR1 = 2000225
EOBJ_CAPTAINS_QUARTERS_DOOR = 2000227
EOBJ_WAVERIDER_GATE = 2000231
EOBJ_THE_HOLE_DOOR = 2000232
EOBJ_KEY_TO_THE_HOLE = 2000256

GIMMICK_BLUE_CORAL_FORMATION = 23
GIMMICK_RED_CORAL_FORMATION = 24
GIMMICK_GREEN_CORAL_FORMATION = 25
GIMMICK_INCONSPICUOUS_SWITCH = 26
GIMMICK_CAPTAINS_QUARTERS_DOOR = 28
GIMMICK_WAVERIDER_GATE = 32
GIMMICK_THE_HOLE = 33
GIMMICK_KEY_TO_THE_HOLE = 40

SEQ0 = 0 -- Activate the coral trigger
SEQ1 = 1 -- Open the hidden door
SEQ2 = 2 -- Discover the pirate captain
SEQ3 = 4 -- Obtain the Waverider Gate key
SEQ4 = 8 -- ???

-- Randomized coral color
local coral_color
-- Whether the party has the key to The Hole
local has_hole
-- Whether the party has the key to Captain's Quarters
local has_captains_quarters
-- Whether the party has the key to Waverider Gate
local has_waverider_gate

function onSetup(director)
    beginSequence0(director)
end

function sequence(director)
    return director:data(0)
end

function setSequence(director, sequence)
    director:set_data(0, sequence)
end

function onGimmickAccessor(director, id)
    if sequence(director) == SEQ0 then
        -- Index to gimmick ID
        GIMMICK_CORAL_IDS = {
            GIMMICK_BLUE_CORAL_FORMATION,
            GIMMICK_RED_CORAL_FORMATION,
            GIMMICK_GREEN_CORAL_FORMATION
        }

        -- Index to EObj ID
        EOBJ_CORAL_IDS = {
            EOBJ_BLUE_CORAL_FORMATION,
            EOBJ_RED_CORAL_FORMATION,
            EOBJ_GREEN_CORAL_FORMATION
        }

        local coral_gimmick_id = GIMMICK_CORAL_IDS[coral_color + 1]

        print("Expecting "..coral_gimmick_id.. " and got "..id)

        director:hide_eobj(EOBJ_CORAL_IDS[id - 22])

        if id == coral_gimmick_id then
            beginSequence1(director)
        end
    elseif sequence(director) == SEQ1 then
        beginSequence2(director)
    end

    if id == GIMMICK_KEY_TO_THE_HOLE then
        -- TODO: play eventaction or whatever
        director:hide_eobj(EOBJ_KEY_TO_THE_HOLE)
        has_hole_key = true
    elseif id == GIMMICK_THE_HOLE and has_hole_key then
        -- TODO: what happens if they don't have access to The Hole?
        director:hide_eobj(EOBJ_THE_HOLE_DOOR)
    elseif id == GIMMICK_CAPTAINS_QUARTERS_DOOR and has_captains_quarters then
        director:hide_eobj(EOBJ_CAPTAINS_QUARTERS_DOOR)
    elseif id == GIMMICK_WAVERIDER_GATE and has_waverider_gate then
        director:hide_eobj(EOBJ_WAVERIDER_GATE)
    end
end

function beginSequence0(director)
    setSequence(director, SEQ0)

    coral_color = math.random(0, 2)
    print("Coral color: "..coral_color)

    hideBloodyMemos(director)
    director:hide_eobj(EOBJ_INCONSPICUOUS_SWITCH)
end

function hideBloodyMemos(director)
    -- TODO: actually delete these eobjs
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

    -- NOTE: Immediately beginning because there's a gatekeeper for this which we don't spawn yet
    beginSequence3(director)
end

function beginSequence3(director)
    setSequence(director, SEQ1 | SEQ2 | SEQ3)

    director:hide_eobj(EOBJ_NEXT_DOOR)
    director:hide_eobj(EOBJ_RAMBADE_DOOR1)
end
