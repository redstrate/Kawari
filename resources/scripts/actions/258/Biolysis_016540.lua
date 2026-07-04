-- 蛊毒法 / Biolysis (SCH, ClassJob 28) — the level-72 upgrade of Bio (17864), a magical
-- damage-over-time. This is the action a max-level Scholar actually casts; the client sends 16540,
-- not 17864, so without this script the server logs "Action 16540 isn't scripted yet!".
-- Status id 1895 (蛊毒法 DoT, "体力逐渐减少").
-- Potency from the ActionTransient tooltip: <if([gnum68=28],<if([gnum72>=94],85,70)>,70)>
--   i.e. SCH level >= 94 -> 85/tick, level 72-93 -> 70/tick.
-- Duration 30s. gain_dot(effect_id, param, duration, per_tick_potency).
BIOLYSIS_STATUS = 1895
DURATION = 30.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Scale per-tick potency by the caster's level (Enhanced Biolysis at 94+).
    local potency = 70
    if player:get_level() >= 94 then
        potency = 85
    end
    -- Biolysis is a pure DoT (no upfront hit), so no damage() here.
    effects:gain_dot(BIOLYSIS_STATUS, 0, DURATION, potency)

    return effects
end
