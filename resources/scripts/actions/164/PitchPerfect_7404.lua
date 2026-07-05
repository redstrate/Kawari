-- Pitch Perfect (BRD, ClassJob 23) - Level 52 ability
-- Potency: 100 per stack (1-3 stacks from Repertoire procs)
-- Recast: 1s (CooldownGroup 1)
-- Requires: Wanderer's Minuet active and Repertoire proc
-- Consumes all Repertoire stacks
POTENCY_PER_STACK = 100
REPERTOIRE_STATUS = 3137

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- For simplicity, use base potency (stack count would be tracked in BardState)
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY_PER_STACK))
    effects:lose_effect(REPERTOIRE_STATUS, 0)

    return effects
end
