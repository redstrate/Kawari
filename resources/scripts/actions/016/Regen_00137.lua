-- 再生 / Regen (WHM/AST) — applies a heal-over-time status on the caster's target.
-- gain_hot applies the status (id 158) AND registers a per-tick heal resolved every 3 seconds.
-- Potency ("恢复力：250" at L85+) from the ActionTransient tooltip. Currently the HoT lands on the
-- caster (on_self); targeted HoTs on party members would need target resolution like other heals.
REGEN_STATUS = 158
HOT_POTENCY = 250
DURATION = 18.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_hot(REGEN_STATUS, 0, DURATION, HOT_POTENCY)

    return effects
end
