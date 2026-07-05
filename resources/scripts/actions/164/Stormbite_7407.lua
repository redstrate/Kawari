-- Stormbite (BRD, ClassJob 23) - Level 64 weaponskill
-- Initial potency: 100
-- DoT potency: 25 per tick, duration 45s
-- Applies status 1322 (Stormbite)
STORMBITE_STATUS = 1201
DOT_DURATION = 45.0
INITIAL_POTENCY = 100
DOT_POTENCY = 25

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Initial hit
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(INITIAL_POTENCY))
    -- Apply DoT (physical damage over time)
    effects:gain_dot_physical(STORMBITE_STATUS, 0, DOT_DURATION, DOT_POTENCY)

    return effects
end
