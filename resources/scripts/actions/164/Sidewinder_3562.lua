-- Sidewinder (BRD, ClassJob 23) - Level 60 ability
-- Potency: 320 (single target), 240 (if target has both DoTs)
-- Recast: 60s (CooldownGroup 13)
POTENCY = 320
POTENCY_WITH_DOTS = 240

function doAction(player, in_combo)
    effects = EffectsBuilder()
    -- For simplicity, use base potency (checking DoT status would require server-side logic)
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_PIERCING, player.parameters:calc_physical_damage(POTENCY))

    return effects
end
