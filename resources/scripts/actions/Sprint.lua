function doAction(player)
    effects = EffectsBuilder()

    -- TODO: go through effectsbuilder
    -- give sprint
    player:give_status_effect(50, 5.0)

    return effects
end
