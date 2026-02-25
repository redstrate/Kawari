-- This file should only be used for globally useful constants and functions.
-- Please put new events, actions, items, etc. in their respective 'main' Lua files.

function split(input, separator)
    if separator == nil then
        separator = '%s'
    end

    local t = {}
    for str in string.gmatch(input, '([^'..separator..']+)') do
        table.insert(t, str)
    end

    return t
end

function getTableSize(tbl)
    local count = 0

    for _, _ in pairs(tbl) do
        count = count + 1
    end

    return count
end

function printf(player, fmt_str, ...)
    -- Sender would be defined elsewhere, if at all
    if command_sender == nil then
        command_sender = ""
    end

    if ... ~= nil then
        player:send_message(command_sender..fmt_str:format(...))
    else
        player:send_message(command_sender..fmt_str)
    end
end

function has_value(tab, val)
    for index, value in ipairs(tab) do
        if value == val then
            return true
        end
    end

    return false
end

-- Constants, if two or more scripts share the same global they should be placed here
EFFECT_TRANSFIGURATION = 565

-- 2 should be the initial opening?
OPENING_CUTSCENE = 2

-- As seen on retail
INITIAL_CUTSCENE_FLAGS = NO_DEFAULT_CAMERA | INVIS_ENPC | CONDITION_CUTSCENE | HIDE_UI | HIDE_HOTBAR | SILENT_ENTER_TERRI_ENV | SILENT_ENTER_TERRI_BGM | SILENT_ENTER_TERRI_SE | DISABLE_SKIP | DISABLE_STEALTH

TERRITORY_S1T2 = 129 -- Limsa Lominsa Lower Decks
TERRITORY_W1T1 = 130 -- Ul'dah - Steps of Nald
TERRITORY_F1T1 = 132 -- New Gridania
TERRITORY_S1H1 = 339 -- Mist
TERRITORY_F1H1 = 340 -- The Lavender Beds
TERRITORY_W1H1 = 341 -- The Goblet
TERRITORY_R2T1 = 418 -- Foundation
TERRITORY_E3T1 = 628 -- Kugane
TERRITORY_E1H1 = 641 -- Shirogane
TERRITORY_R1H1 = 979 -- Empyreum

