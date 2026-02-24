-- Sastasha

EOBJ_BLOODY_MEMO_BLUE = 2000212
EOBJ_BLOODY_MEMO_RED = 2001548
EOBJ_BLOODY_MEMO_GREEN = 2001549
EOBJ_INCONSPICUOUS_SWITCH = 2000216
-- TODO: these are *technically* duplicating the IDs below, and that should probably be fixed
EOBJ_BLUE_CORAL_FORMATION = 2000213
EOBJ_RED_CORAL_FORMATION = 2000214
EOBJ_GREEN_CORAL_FORMATION = 2000215

GIMMICK_BLUE_CORAL_FORMATION = 23
GIMMICK_RED_CORAL_FORMATION = 24
GIMMICK_GREEN_CORAL_FORMATION = 25
GIMMICK_INCONSPICUOUS_SWITCH = 26

SEQ0 = 0
SEQ1 = 1

-- Randomized coral color
local coral_color

function onSetup(director)
    beginSequence0(director)
end

function onGimmickAccessor(director, id)
    -- Index to gimmick ID
    GIMMICK_CORAL_IDS = {
        GIMMICK_BLUE_CORAL_FORMATION,
        GIMMICK_RED_CORAL_FORMATION,
        GIMMICK_GREEN_CORAL_FORMATION
    }

    local coral_gimmick_id = GIMMICK_CORAL_IDS[coral_color + 1]

    print("Expecting "..coral_gimmick_id.. " and got "..id)

    if id == coral_gimmick_id then
        beginSequence1(director)
    end
end

function beginSequence0(director)
    director:set_data(0, SEQ0)

    coral_color = math.random(0, 2)
    print("Coral color: "..coral_color)

    hideBloodyMemos(director)
    hideInconspicuousSwitch(director)
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

function hideInconspicuousSwitch(director)
    director:hide_eobj(EOBJ_INCONSPICUOUS_SWITCH)
end

function beginSequence1(director)
    director:set_data(0, SEQ1)

    director:delete_eobj(EOBJ_BLUE_CORAL_FORMATION)
    director:delete_eobj(EOBJ_RED_CORAL_FORMATION)
    director:delete_eobj(EOBJ_GREEN_CORAL_FORMATION)

    showInconspicuousSwitch(director)
end

function showInconspicuousSwitch(director)
    director:show_eobj(EOBJ_INCONSPICUOUS_SWITCH)
end
