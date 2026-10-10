POTENCY = 380
STATUS_AETHERHUES = 3675

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_TYPE_MAGIC, player.parameters:calc_magical_damage(POTENCY))
    effects:gain_effect_self(STATUS_AETHERHUES, 0, 30.0)

    return effects
end
