STATUS_RAPTOR_FORM = 108

function doAction(player)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, DAMAGE_ELEMENT_UNASPECTED, 180)
    effects:gain_effect(STATUS_RAPTOR_FORM, 0, 30.0)

    return effects
end
