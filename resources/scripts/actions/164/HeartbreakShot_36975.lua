-- Heartbreak Shot (BRD, ClassJob 23) - Level 92 ability
-- Potency: 180 (single target)
-- Recast: 15s (CooldownGroup 10)
POTENCY = 180

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
