POTENCY = 180
STATUS_RAPTOR_FORM = 108

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_SLASHING, player.parameters:calc_physical_damage(POTENCY))
    effects:gain_effect_self(STATUS_RAPTOR_FORM, 0, 30.0)

    return effects
end
