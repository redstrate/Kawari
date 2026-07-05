-- Refulgent Arrow (BRD, ClassJob 23) - Level 70 weaponskill
-- Potency: 280
-- Requires: Hawk Eye status (3861) from Barrage or song procs
POTENCY = 280
HAWK_EYE_STATUS = 3861
RESONANT_ARROW_READY_STATUS = 3862

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))
    effects:lose_effect(HAWK_EYE_STATUS, 0)
    effects:lose_effect(RESONANT_ARROW_READY_STATUS, 0)

    return effects
end
