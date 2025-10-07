DBG_DIR = "commands/debug/"
GM_DIR = "commands/gm/"

-- GM commands

GM_SET_POS = 0
GM_SET_LEVEL = 1
GM_CHANGE_WEATHER = 6
GM_SPEED = 9
GM_INVISIBILITY = 13
GM_SET_HP = 100
GM_SET_MP = 101
GM_EXP = 104
GM_ORCHESTRION = 116
GM_GIVE_ITEM = 200
GM_GIL = 201
GM_COLLECT = 202
GM_AETHERYTE = 350
GM_INSPECT = 422
GM_WIREFRAME = 550
GM_TERRITORY = 600
GM_TERRITORY_INFO = 605

registerGMCommand(GM_SET_POS,           GM_DIR.."SetPos.lua")
registerGMCommand(GM_SET_LEVEL,         GM_DIR.."SetLevel.lua")
registerGMCommand(GM_CHANGE_WEATHER,    GM_DIR.."ChangeWeather.lua")
registerGMCommand(GM_SPEED,             GM_DIR.."SetSpeed.lua")
registerGMCommand(GM_INVISIBILITY,      GM_DIR.."ToggleInvisibility.lua")
registerGMCommand(GM_SET_HP,            GM_DIR.."SetHP.lua")
registerGMCommand(GM_SET_MP,            GM_DIR.."SetMP.lua")
-- TODO: Implement the GMInspect IPC opcode, it's completely different than the normal GMCommand opcode
--registerGMCommand(GM_INSPECT,           GM_DIR.."InspectPlayer.lua")
registerGMCommand(GM_EXP,               GM_DIR.."Exp.lua")
registerGMCommand(GM_ORCHESTRION,       GM_DIR.."Orchestrion.lua")
registerGMCommand(GM_GIVE_ITEM,         GM_DIR.."GiveItem.lua")
registerGMCommand(GM_GIL,               GM_DIR.."Gil.lua")
registerGMCommand(GM_COLLECT,           GM_DIR.."Collect.lua")
registerGMCommand(GM_AETHERYTE,         GM_DIR.."UnlockAetheryte.lua")
registerGMCommand(GM_WIREFRAME,         GM_DIR.."ToggleWireframe.lua")
registerGMCommand(GM_TERRITORY,         GM_DIR.."ChangeTerritory.lua")
registerGMCommand(GM_TERRITORY_INFO,    GM_DIR.."TerritoryInfo.lua")

-- Debug commands
-- Please keep these in alphabetical order!

registerCommand("classjob",                         DBG_DIR.."ClassJob.lua")
registerCommand("festival",                         DBG_DIR.."Festival.lua")
registerCommand("inspect",                          GM_DIR.."InspectPlayer.lua") -- TODO: remove this once we figure out the GMInspect IPC opcode
registerCommand("nudge",                            DBG_DIR.."Nudge.lua")
registerCommand("ost",                              DBG_DIR.."OnScreenTest.lua")
registerCommand("permtest",                         DBG_DIR.."PermissionTest.lua")
registerCommand("toggleadventure",                  DBG_DIR.."ToggleAdventure.lua")
registerCommand("toggleaethercurrent",              DBG_DIR.."ToggleAetherCurrent.lua")
registerCommand("toggleaethercurrentcompflgset",    DBG_DIR.."ToggleAetherCurrentCompFlgSet.lua")
registerCommand("togglecaughtfish",                 DBG_DIR.."ToggleCaughtFish.lua")
registerCommand("togglecaughtspearfish",            DBG_DIR.."ToggleCaughtSpearfish.lua")
registerCommand("togglechocobotaxistand",           DBG_DIR.."ToggleChocoboTaxiStand.lua")
registerCommand("togglecutsceneseen",               DBG_DIR.."ToggleCutsceneSeen.lua")
registerCommand("toggleglassesstyle",               DBG_DIR.."ToggleGlassesStyle.lua")
registerCommand("toggleminion",                     DBG_DIR.."ToggleMinion.lua")
registerCommand("togglemount",                      DBG_DIR.."ToggleMount.lua")
registerCommand("toggleornament",                   DBG_DIR.."ToggleOrnament.lua")
registerCommand("toggletripletriadcard",            DBG_DIR.."ToggleTripleTriadCard.lua")
registerCommand("unlock",                           DBG_DIR.."Unlock.lua")
registerCommand("unlockbuddyequip",                 DBG_DIR.."UnlockBuddyEquip.lua")
registerCommand("unlockcontent",                    DBG_DIR.."UnlockContent.lua")
registerCommand("completeallquests",                DBG_DIR.."CompleteAllQuests.lua")

