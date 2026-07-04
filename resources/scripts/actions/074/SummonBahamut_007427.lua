-- 龙神召唤 / Summon Bahamut (enables the Bahamut burst phase)
-- Simplified: no demi pet/timer is modelled yet, so this just plays as an instant GCD with no
-- direct effect. The follow-up burst skills (龙神迸发/死星核爆) are not gated server-side.
function doAction(player, in_combo)
    effects = EffectsBuilder()
    effects:summon_demi()

    return effects
end
