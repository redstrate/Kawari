-- Provoke: puts the caster at the top of the target's hate list (highest enmity + 1).
-- This is a pure enmity action with no damage.

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:provoke()

    return effects
end
