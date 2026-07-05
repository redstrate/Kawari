-- Warden's Paean (BRD, ClassJob 23) - Level 35 ability
-- Effect: Removes one detrimental effect from target. If none, grants a status that blocks the
-- next removable detrimental effect.
-- Status 866 is the PvE Warden's Paean guard. Do not use 3561: that is Target Beta.
WARDENS_PAEAN_STATUS = 866
DURATION = 30.0

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:gain_effect(WARDENS_PAEAN_STATUS, 0, DURATION)

    return effects
end
