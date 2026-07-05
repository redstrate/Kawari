-- Empyreal Arrow (BRD, ClassJob 23) - Level 54 ability
-- Potency: 240
-- Recast: 15s (CooldownGroup 3)
-- Does not share a recast timer with other weaponskills
POTENCY = 240

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
