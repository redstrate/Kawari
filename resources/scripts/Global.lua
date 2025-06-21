function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

function onCommandRequiredRankInsufficientError(player)
    player:send_message("You do not have permission to run this command.")
end

function onCommandRequiredRankMissingError(additional_information, player)
    local error_msg = "Your script does not define the required_rank variable. Please define it in your script for it to run."

    player:send_message(string.format("%s\nAdditional information: %s", error_msg, additional_information))
end

function onUnknownCommandError(command_name, player)
    player:send_message(string.format("Unknown command %s", command_name))
end

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

-- Constants
GM_RANK_NORMALUSER = 0
GM_RANK_GAMEMASTER = 1
GM_RANK_EVENTJUNIOR = 3
GM_RANK_EVENTSENIOR = 4
GM_RANK_SUPPORT = 5
GM_RANK_SENIOR = 7
GM_RANK_DEBUG = 90
GM_RANK_MAX = 255 -- Doesn't exist, used for purposes of testing permissions in scripts


-- please keep these ids sorted!

-- Actions
registerAction(3, "actions/Sprint.lua")
registerAction(5, "actions/Teleport.lua")
registerAction(9, "actions/FastBlade.lua")

-- Items
registerAction(6221, "items/Fantasia.lua")

-- Events
registerEvent(131079, "warp/WarpInnGeneric.lua") -- Exit Limsa Upper Decks to Mizzenmast Inn room
registerEvent(131080, "warp/WarpInnGeneric.lua") -- Exit New Gridania to The Roost Inn room
registerEvent(131081, "warp/WarpInnGeneric.lua") -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
registerEvent(131082, "common/GenericWarp.lua") -- Exit Mizzenmast Inn to Limsa Upper Decks
registerEvent(131083, "common/GenericWarp.lua") -- Exit The Roost to New Gridania
registerEvent(131084, "common/GenericWarp.lua") -- Exit The Hourglass to Ul'dah: Steps of Nald
registerEvent(131092, "common/GenericWarp.lua")
registerEvent(131093, "common/GenericWarp.lua")
registerEvent(131094, "common/GenericWarp.lua")
--registerEvent(131113, "common/GenericWarp.lua") -- (currently doesn't react, seems to need different handling) Ferry Skipper from Upper La Noscea: Memeroon's Trading Post to Upper La Noscea: Jijiroon's Trading Post
--registerEvent(131114, "common/GenericWarp.lua") -- (currently doesn't react, seems to need different handling) Ferry Skipper from Upper La Noscea: Jijiroon's Trading Post to Upper La Noscea: Memeroon's Trading Post
registerEvent(131126, "common/GenericWarp.lua") -- Gatekeeper from Southern Thanalan: Nald's Reflection to Southern Thanalan: The Minotaur Malm
registerEvent(131204, "warp/WarpInnGeneric.lua") -- Exit Ishgard: Foundation to Cloud Nine Inn room
registerEvent(131205, "common/GenericWarp.lua") -- Exit Cloud Nine to Ishgard: Foundation
registerEvent(131246, "common/GenericWarp.lua") -- Exit Bokairo Inn to Kugane
registerEvent(131250, "common/GenericWarp.lua") -- Gatekeeper from The Fringes: Castrum Oriens to East Shroud: Amarissaaix's Spire
registerEvent(131312, "common/GenericWarp.lua") -- Exit The Pendants Personal Suite to Crystarium
registerEvent(131402, "common/GenericWarp.lua") -- Exit Andron to Old Sharlayan
registerEvent(131428, "common/GenericWarp.lua") -- Mothercrystal Exit (note: warp doesn't work for some reason?)
registerEvent(131519, "common/GenericWarp.lua") -- Faire Adventurer from Eastern La Noscea: bottom of the Moonfire Festival (2023) tower to the first tier of the tower
registerEvent(131578, "common/GenericWarp.lua") -- Exit The For'ard Cabins to Tuliyollal
-- A Realm Reborn Aetherytes
registerEvent(327683, "common/GenericAetheryte.lua") -- Bentbranch Meadows Aetheryte
registerEvent(327684, "common/GenericAetheryte.lua") -- The Hawthorne Hut Aetheryte
registerEvent(327685, "common/GenericAetheryte.lua") -- Quarrymill Aetheryte
registerEvent(327686, "common/GenericAetheryte.lua") -- Camp Tranquil Aetheryte
registerEvent(327687, "common/GenericAetheryte.lua") -- Fallgourd Float Aetheryte
registerEvent(327690, "common/GenericAetheryte.lua") -- Moraby Drydocks Aetheryte
registerEvent(327691, "common/GenericAetheryte.lua") -- Costa del Sol Aetheryte
registerEvent(327692, "common/GenericAetheryte.lua") -- Wineport Aetheryte
-- registerEvent(327???, "common/GenericAetheryte.lua") -- Wineport Aetheryte, currently unknown due to only being able to load ELN once. Now it loads into purgatory...
registerEvent(327693, "common/GenericAetheryte.lua") -- Swiftperch Aetheryte
registerEvent(327694, "common/GenericAetheryte.lua") -- Aleport Aetheryte
registerEvent(327695, "common/GenericAetheryte.lua") -- Camp Bronze Lake Aetheryte
registerEvent(327696, "common/GenericAetheryte.lua") -- Camp Overlook Aetheryte
registerEvent(327697, "common/GenericAetheryte.lua") -- Horizon Aetheryte
registerEvent(327698, "common/GenericAetheryte.lua") -- Camp Drybone Aetheryte
registerEvent(327699, "common/GenericAetheryte.lua") -- Little Ala Mhigo Aetheryte
registerEvent(327700, "common/GenericAetheryte.lua") -- Forgotten Springs Aetheryte
registerEvent(327701, "common/GenericAetheryte.lua") -- Camp Bluefog Aetheryte
registerEvent(327702, "common/GenericAetheryte.lua") -- Ceruleum Processing Plant Aetheryte
registerEvent(327703, "common/GenericAetheryte.lua") -- Camp Dragonhead Aetheryte
registerEvent(327732, "common/GenericAetheryte.lua") -- Summerford Farms Aetheryte
registerEvent(327733, "common/GenericAetheryte.lua") -- Black Brush Station Aetheryte
registerEvent(327735, "common/GenericAetheryte.lua") -- Wolves' Den Pier Aetheryte
-- registerevent(327???, "common/GenericAetheryte.lua") -- Ul'dah: Steps of Nald Aetheryte, currently unknown due to the entity not spawning
-- registerevent(327???, "common/GenericAetheryte.lua") -- Limsa Lominsa: The Lower Decks Aetheryte, currently unknown due to the entity not spawning
registerEvent(327742, "common/GenericAetheryte.lua") -- The Gold Saucer Aetheryte

-- Heavensward Aetherytes
registerEvent(327750, "common/GenericAetheryte.lua") -- Ishgard: Foundation Aetheryte
registerEvent(327751, "common/GenericAetheryte.lua") -- Falcon's Nest Aetheryte
registerEvent(327752, "common/GenericAetheryte.lua") -- Camp Cloudtop Aetheryte
registerEvent(327753, "common/GenericAetheryte.lua") -- Ok' Zundu Aetheryte
registerEvent(327754, "common/GenericAetheryte.lua") -- Helix Aetheryte
registerEvent(327755, "common/GenericAetheryte.lua") -- Idyllshire Aetheryte
registerEvent(327756, "common/GenericAetheryte.lua") -- Tailfeather Aetheryte
registerEvent(327757, "common/GenericAetheryte.lua") -- Anyx Trine Aetheryte
registerEvent(327758, "common/GenericAetheryte.lua") -- Moghome Aetheryte
registerEvent(327759, "common/GenericAetheryte.lua") -- Zenith Aetheryte

-- Stormblood Aetherytes
registerEvent(327778, "common/GenericAetheryte.lua") -- Castrum Oriens Aetheryte
registerEvent(327779, "common/GenericAetheryte.lua") -- The Peering Stones Aetheryte
registerEvent(327780, "common/GenericAetheryte.lua") -- Ala Gannha Aetheryte
registerEvent(327781, "common/GenericAetheryte.lua") -- Ala Ghiri Aetheryte
registerEvent(327782, "common/GenericAetheryte.lua") -- Porta Praetoria Aetheryte
registerEvent(327783, "common/GenericAetheryte.lua") -- The Ala Mhigan Quarter Aetheryte
registerEvent(327784, "common/GenericAetheryte.lua") -- Rhalgr's Reach Aetheryte
registerEvent(327785, "common/GenericAetheryte.lua") -- Tamamizu Aetheryte
registerEvent(327786, "common/GenericAetheryte.lua") -- Onokoro Aetheryte
registerEvent(327787, "common/GenericAetheryte.lua") -- Namai Aetheryte
registerEvent(327788, "common/GenericAetheryte.lua") -- The House of the Fierce Aetheryte
-- registerEvent(3277??, "common/GenericAetheryte.lua") -- Reunion Aetheryte, all Azim Steppe aetherytes cannot be accessed because the client thinks it's underwater and throws the error "Cannot execute command. Environment different."
-- registerEvent(3277??, "common/GenericAetheryte.lua") -- The Dawn Throne Aetheryte
-- registerEvent(3277??, "common/GenericAetheryte.lua") -- Dhoro Iloh Aetheryte
registerEvent(327791, "common/GenericAetheryte.lua") -- Kugane Aetheryte
registerEvent(327807, "common/GenericAetheryte.lua") -- The Doman Enclave Aetheryte

-- Shadowbringers Aetherytes
registerEvent(327812, "common/GenericAetheryte.lua") -- Fort Jobb Aetheryte
registerEvent(327814, "common/GenericAetheryte.lua") -- Eulmore Aetheryte
registerEvent(327816, "common/GenericAetheryte.lua") -- The Ostal Imperative Aetheryte
registerEvent(327817, "common/GenericAetheryte.lua") -- Stilltide Aetheryte
registerEvent(327818, "common/GenericAetheryte.lua") -- Wright Aetheryte
registerEvent(327819, "common/GenericAetheryte.lua") -- Tomra Aetheryte
registerEvent(327820, "common/GenericAetheryte.lua") -- Mord Souq Aetheryte
registerEvent(327821, "common/GenericAetheryte.lua") -- Twine Aetheryte
registerEvent(327822, "common/GenericAetheryte.lua") -- Slitherbough Aetheryte
registerEvent(327823, "common/GenericAetheryte.lua") -- Fanow Aetheryte
registerEvent(327824, "common/GenericAetheryte.lua") -- Lydha Lran Aetheryte
registerEvent(327825, "common/GenericAetheryte.lua") -- Pla Enni Aetheryte
registerEvent(327826, "common/GenericAetheryte.lua") -- Wolekdorf Aetheryte
registerEvent(327827, "common/GenericAetheryte.lua") -- The Ondo Cups Aetheryte
registerEvent(327828, "common/GenericAetheryte.lua") -- The Macarenses Angle Aetheryte
registerEvent(327841, "common/GenericAetheryte.lua") -- The Inn at Journey's Head Aetheryte
-- registerEvent(3278??, "common/GenericAetheryte.lua") -- The Crystarium Aetheryte, currently unknown due to the entity not spawning

-- Endwalker Aetherytes
registerEvent(327846, "common/GenericAetheryte.lua") -- The Archeion Aetheryte
registerEvent(327847, "common/GenericAetheryte.lua") -- Sharlayan Hamlet Aetheryte
registerEvent(327848, "common/GenericAetheryte.lua") -- Aporia Aetheryte
registerEvent(327849, "common/GenericAetheryte.lua") -- Yedlihmad Aetheryte
registerEvent(327850, "common/GenericAetheryte.lua") -- The Great Work Aetheryte
registerEvent(327851, "common/GenericAetheryte.lua") -- Palaka's Stand Aetheryte
registerEvent(327852, "common/GenericAetheryte.lua") -- Camp Broken Glass Aetheryte
registerEvent(327853, "common/GenericAetheryte.lua") -- Tertium Aetheryte
registerEvent(327854, "common/GenericAetheryte.lua") -- Sinus Lacrimarum Aetheryte
registerEvent(327855, "common/GenericAetheryte.lua") -- Bestways Burrow Aetheryte
registerEvent(327856, "common/GenericAetheryte.lua") -- Anagnorisis Aetheryte
registerEvent(327857, "common/GenericAetheryte.lua") -- The Twelve Wonders Aetheryte
registerEvent(327858, "common/GenericAetheryte.lua") -- Poieten Oikos Aetheryte
registerEvent(327859, "common/GenericAetheryte.lua") -- Reah Tahra Aetheryte
registerEvent(327860, "common/GenericAetheryte.lua") -- Abode of the Ea Aetheryte
registerEvent(327861, "common/GenericAetheryte.lua") -- Base Omicron Aetheryte
registerEvent(327862, "common/GenericAetheryte.lua") -- Old Sharlayan Aetheryte
registerEvent(327863, "common/GenericAetheryte.lua") -- Radz-at-Han Aetheryte

-- Dawntrail Aetherytes
registerEvent(327880, "common/GenericAetheryte.lua") -- Wachunpelo Aetheryte
registerEvent(327881, "common/GenericAetheryte.lua") -- Worlar's Echo Aetheryte
registerEvent(327882, "common/GenericAetheryte.lua") -- Ok'hanu Aetheryte
registerEvent(327883, "common/GenericAetheryte.lua") -- Many Fires Aetheryte
registerEvent(327884, "common/GenericAetheryte.lua") -- Earthenshire Aetheryte
registerEvent(327885, "common/GenericAetheryte.lua") -- Iq Br'aax Aetheryte
registerEvent(327886, "common/GenericAetheryte.lua") -- Mamook Aetheryte
registerEvent(327887, "common/GenericAetheryte.lua") -- Hhusatahwi Aetheryte
registerEvent(327888, "common/GenericAetheryte.lua") -- Sheshenewezi Springs Aetheryte
registerEvent(327889, "common/GenericAetheryte.lua") -- Mehwahhetsoan Aetheryte
registerEvent(327890, "common/GenericAetheryte.lua") -- Yyasulani Station Aetheryte
registerEvent(327891, "common/GenericAetheryte.lua") -- The Outskirts Aetheryte
registerEvent(327892, "common/GenericAetheryte.lua") -- Electrope Strike Aetheryte
registerEvent(327893, "common/GenericAetheryte.lua") -- Leynode Mnemo Aetheryte
registerEvent(327894, "common/GenericAetheryte.lua") -- Leynode Pyro Aetheryte
registerEvent(327895, "common/GenericAetheryte.lua") -- Leynode Aero Aetheryte
registerEvent(327896, "common/GenericAetheryte.lua") -- Tuliyollal Aetheryte
registerEvent(327897, "common/GenericAetheryte.lua") -- Solution Nine Aetheryte
registerEvent(327918, "common/GenericAetheryte.lua") -- Dock Poga Aetheryte

-- Misc. Events
registerEvent(720916, "custom/000/cmndefinnbed_00020.lua")
registerEvent(721028, "tosort/UnendingJourney.lua")
registerEvent(721044, "tosort/CrystalBell.lua")
registerEvent(1179657, "tosort/Chocobokeep.lua") -- Chocobokeep in Central Shroud
registerEvent(1245185, "opening/OpeningLimsaLominsa.lua")
registerEvent(1245186, "opening/OpeningGridania.lua")
registerEvent(1245187, "opening/OpeningUldah.lua")

-- TODO: Generic warps might be decided through ArrayEventHandler?

-- Commands
registerCommand("setpos", "commands/debug/SetPos.lua")
registerCommand("classjob", "commands/debug/ClassJob.lua")
registerCommand("setspeed", "commands/debug/SetSpeed.lua")
registerCommand("nudge", "commands/debug/Nudge.lua")
registerCommand("festival", "commands/debug/Festival.lua")
registerCommand("permtest", "commands/debug/PermissionTest.lua")
registerCommand("reload", "commands/debug/Reload.lua")
registerCommand("unlockaction", "commands/debug/UnlockAction.lua")
registerCommand("wireframe", "commands/debug/ToggleWireframe.lua")
registerCommand("invis", "commands/debug/ToggleInvisibility.lua")
registerCommand("unlockaetheryte", "commands/debug/UnlockAetheryte.lua")
registerCommand("teri", "commands/debug/ChangeTerritory.lua")
