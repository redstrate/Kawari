function doAction(player, arg)
    effects = EffectsBuilder()

    -- TODO: match retail fantasia behavior
    player:set_remake_mode("EditAppearance")

    return effects
end
