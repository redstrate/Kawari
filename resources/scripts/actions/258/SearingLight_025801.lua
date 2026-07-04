-- 灼热之光 / Searing Light (party damage buff; self-applied in this simplified model)
SEARING_LIGHT = 2703

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect_self(SEARING_LIGHT, 0, 20.0)

    return effects
end
