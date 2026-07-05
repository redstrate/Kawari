-- Heavy Shot (ARC/BRD, ClassJob 5/23) - Level 1 weaponskill
-- Potency: 180 (before Burst Shot upgrade at level 76)
POTENCY = 180

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
