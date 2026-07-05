-- Apex Arrow (BRD, ClassJob 23) - Level 80 weaponskill (AoE)
-- Potency: 500 (main target), 250 (secondary targets)
-- Consumes all Soul Voice gauge (requires 80+ to use)
-- At level 86+, grants Blast Arrow Ready status
MAIN_POTENCY = 500
AOE_POTENCY = 250
BLAST_ARROW_READY_STATUS = 2692
BLAST_ARROW_READY_DURATION = 10.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(MAIN_POTENCY))

    -- At level 86+, Apex Arrow grants Blast Arrow Ready
    if player:get_level() >= 86 then
        effects:gain_effect_self(BLAST_ARROW_READY_STATUS, 0, BLAST_ARROW_READY_DURATION)
    end
    effects:modify_gauge(0, -100)

    return effects
end
