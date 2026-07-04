-- 毒菌 / Bio (SCH, ClassJob 28) — applies a magical damage-over-time status.
-- Action 17864 is the CURRENT live Bio (Action.ClassJob = 28 / IsPlayerAction = true). The old
-- action 164 is deprecated (Action.ClassJob = -1, IsPlayerAction = false) — never use ids whose
-- ClassJob is -1, those are unassigned/role-play NPC actions.
-- Status id 179, DoT potency 20/tick over 30s (ActionTransient tooltip "威力：20").
-- gain_dot(effect_id, param, duration, per_tick_potency).
BIO_STATUS = 179
DOT_POTENCY = 20
DURATION = 30.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Bio is a pure DoT (no upfront hit), so no damage() here.
    effects:gain_dot(BIO_STATUS, 0, DURATION, DOT_POTENCY)

    return effects
end
