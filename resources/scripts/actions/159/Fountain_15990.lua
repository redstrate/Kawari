POTENCY = 120 -- TODO: has a combo potency of 280

EFFECT_SILKEN_SYMMETRY = 2693

function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
