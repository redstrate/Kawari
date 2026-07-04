POTENCY = 460

function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:damage(DAMAGE_KIND_NORMAL, DAMAGE_TYPE_MAGIC, player.parameters:calc_magical_damage(POTENCY))
    -- Spends 1 Aetherflow stack (the action is gated on having one server-side).
    effects:modify_gauge(GAUGE_AETHERFLOW, -1)

    return effects
end
