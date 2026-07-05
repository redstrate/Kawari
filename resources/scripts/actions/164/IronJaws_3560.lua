-- Iron Jaws (BRD, ClassJob 23) - Level 56 weaponskill
-- Potency: 100
-- Refreshes the duration of both Caustic Bite and Stormbite on the target
-- Requires at least one DoT to be active
POTENCY = 100
DOT_REFRESH_DURATION = 45.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Damage hit
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))

    -- Refresh both DoTs by reapplying the statuses. The server refreshes same-status DoT slots.
    -- Caustic Bite (1200)
    effects:gain_dot_physical(1200, 0, DOT_REFRESH_DURATION, 20)

    -- Stormbite (1201)
    effects:gain_dot_physical(1201, 0, DOT_REFRESH_DURATION, 25)

    return effects
end
