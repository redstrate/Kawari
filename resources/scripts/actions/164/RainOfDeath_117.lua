-- Rain of Death (BRD, ClassJob 23) - Level 45 ability (AoE)
-- Potency: 100 (all targets in range)
-- Recast: 15s (CooldownGroup 10)
POTENCY = 100

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
