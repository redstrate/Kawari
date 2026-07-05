-- Repelling Shot (BRD, ClassJob 23) - Level 15 ability
-- Effect: Jump backwards 10 yalms
-- Recast: 30s (CooldownGroup 6)
-- No damage, movement ability

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- Repelling Shot is a movement ability with no damage
    -- The actual movement would be handled by the client

    return effects
end
