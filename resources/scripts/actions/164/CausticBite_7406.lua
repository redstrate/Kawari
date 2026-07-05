-- Caustic Bite (BRD, ClassJob 23) - Level 64 weaponskill
-- Initial potency: 150
-- DoT potency: 20 per tick, duration 45s
-- Applies status 1321 (Caustic Bite)
CAUSTIC_BITE_STATUS = 1200
DOT_DURATION = 45.0
INITIAL_POTENCY = 150
DOT_POTENCY = 20

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Initial hit
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(INITIAL_POTENCY))
    -- Apply DoT (physical damage over time)
    effects:gain_dot_physical(CAUSTIC_BITE_STATUS, 0, DOT_DURATION, DOT_POTENCY)

    return effects
end
