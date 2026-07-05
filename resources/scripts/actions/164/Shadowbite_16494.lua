-- Shadowbite (BRD, ClassJob 23) - Level 72 weaponskill (AoE)
-- Potency: 170 (main target), 100 (secondary targets)
-- Requires: Hawk Eye status (3861) or Shadowbite Ready (3002)
MAIN_POTENCY = 170
AOE_POTENCY = 100
HAWK_EYE_STATUS = 3861
SHADOWBITE_READY_STATUS = 3002

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(MAIN_POTENCY))
    effects:lose_effect(HAWK_EYE_STATUS, 0)
    effects:lose_effect(SHADOWBITE_READY_STATUS, 0)

    return effects
end
