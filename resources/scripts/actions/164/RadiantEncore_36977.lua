-- Radiant Encore (BRD, ClassJob 23) - Level 100 ability
-- Potency: 320 (single target)
-- Requires: Radiant Encore Ready status (3863)
-- Recast: 1s (CooldownGroup 1)
POTENCY = 320
RADIANT_ENCORE_READY_STATUS = 3863

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))
    effects:lose_effect(RADIANT_ENCORE_READY_STATUS, 0)

    return effects
end
