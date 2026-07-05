-- Resonant Arrow (BRD, ClassJob 23) - Level 96 ability
-- Potency: 240 (single target)
-- Requires: Resonant Arrow Ready status (3862)
-- Recast: 1s (CooldownGroup 1, shares with Pitch Perfect)
POTENCY = 240
RESONANT_ARROW_READY_STATUS = 3862

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))
    effects:lose_effect(RESONANT_ARROW_READY_STATUS, 0)

    return effects
end
