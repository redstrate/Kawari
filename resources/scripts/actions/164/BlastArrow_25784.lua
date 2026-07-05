-- Blast Arrow (BRD, ClassJob 23) - Level 86 weaponskill (AoE)
-- Potency: 600 (main target), 300 (secondary targets)
-- Requires: Blast Arrow Ready status (2692 or 3142)
MAIN_POTENCY = 600
AOE_POTENCY = 300
BLAST_ARROW_READY_STATUS = 2692
BLAST_ARROW_READY_STATUS_ALT = 3142

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(MAIN_POTENCY))
    effects:lose_effect(BLAST_ARROW_READY_STATUS, 0)
    effects:lose_effect(BLAST_ARROW_READY_STATUS_ALT, 0)

    return effects
end
