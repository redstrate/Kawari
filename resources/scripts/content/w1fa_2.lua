-- 真火神歼灭战 The Bowl of Embers (Hard) / Ifrit — content short name: w1fa_2
--
-- 第一版纵向切片(Step 9 验证):spawn Ifrit + 跑通 地火喷发 / 烈炎喷射 / 光辉炎柱 /
-- 深红旋风 / 地狱之火炎 的 cast -> 结算闭环。暂不做地狱之钉(Infernal Nail)阶段。
--
-- 机制形状/数值参考 BossmodReborn T05IfritH.cs(GroupID=59=w1fa_2, NameID=1185),
-- action id / 读条时间参考 w1fa_2 retail 抓包。

-- TODO: boss 名牌图标显示成蓝色(高阶怪物)而不是红色,待查正确的 BNpcBase/BNpcName(我从抓包解出的
-- 243/1219/468 被确认是错的,先用回能出正确模型的 209/1185/434)。
local IFRIT_BASE_ID = 209
local IFRIT_HELPER_ID = 434
local IFRIT_NAME_ID = 1185      -- BNpcName "伊弗利特"
local IFRIT_HP      = 299314    -- 解限 HP(取自抓包);阶段判定走 HP%,不依赖具体值
local IFRIT_LEVEL   = 50

-- 竞技场中心(已确认 = 原点)。机制点位都相对它。
local ARENA_CENTER = { x = 0.0, y = 0.0, z = 0.0 }

-- Boss 出生点:抓包里主 boss(4000321D)直接 spawn 在 (15,0,0) 东侧、heading -90°(面朝西),
-- 与西侧出生的玩家正对,且 spawn 后不移动(服务器没有二次调位)。
-- 朝向约定 forward = (sin θ, cos θ):0=南(+Z) / π=北(-Z) / π/2=东(+X) / -π/2=西(-X)。
local IFRIT_SPAWN  = { x = 15.0, y = 0.0, z = 0.0 }
local IFRIT_FACING = -math.pi / 2   -- 面朝西,正对玩家

-- 技能(id / 读条 / 形状,均据 BMR + 抓包)
local ACTION_INCINERATE   = 1353   -- 烈炎喷射 boss 正面 120° 扇形, r15(retail 无读条,这里给 1s 反应窗)
local ACTION_ERUPTION     = 1355   -- 地火喷发 预读条 2.2s(单体视觉)
local ACTION_ERUPTION_HIT = 1358   -- 地火喷发 结算:helper r8 圆
local ACTION_PLUME        = 1356   -- 光辉炎柱 预读条 2.2s(单体视觉)
local ACTION_PLUME_HIT    = 1359   -- 光辉炎柱 结算:helper r8 圆
local ACTION_CYCLONE      = 457    -- 深红旋风 3.0s,rect 43 长 × 12 宽(retail 4 条同时)
local ACTION_HELLFIRE     = 1357   -- 地狱之火炎 2.0s,全屏(retail 是钉子阶段 enrage)

-- 地面预警(omen)= 让一个 actor "施放"该 AOE 技能、target=无目标、position=落点,
-- 客户端据技能数据在落点画出 omen 形状(retail 就是这么做的,没有单独的 omen 包)。
-- aoe_*({ omen = true }) 会自动从 omen helper 池(spawn_omen_pool)里取一个 invisible helper
-- 当施法者(round-robin),并把结算特效也从它发 → boss 不被占用/冲走、多个 omen 可同时出。
-- 池为空时退化为用 boss 当施法者。注:omen 形状来自 action id 自带数据,要用带 omen 的真技能 id。

EOBJ_EXIT = 2000139  -- 通关后 complete_duty 会显示的退出点

-- 深红旋风分身的**固定**边缘点:半径 21、相邻 45°,朝向圆心(a+π = forward 朝中心穿场)。
-- 一进本就把分身 spawn 在这些点上、朝向直接定好,放技能时**绝不 re-position**——否则客户端会把
-- 那次 ActorMove lerp 过去,cast 时朝向取到的是 lerp 中途的值,冲刺方向就偏了(这是之前一直错的根因)。
local CYCLONE_R    = 21.0
local CYCLONE_STEP = math.pi / 4.0
-- 第 i 个分身的边缘点 + 朝向(a+π = 朝圆心)。base 是本次旋风的随机起始角(retail 冲刺方向也是随机的);
-- spawn 待机时用 base=0 的固定点,冲锋时传一个随机 base,分身 move 到新点 + 朝向再冲。
local function clone_spot(i, base)
    local a = (base or 0) + (i - 1) * CYCLONE_STEP
    return { x = CYCLONE_R * math.sin(a), y = 0.0, z = CYCLONE_R * math.cos(a) }, a + math.pi
end

-- omen 施法者池 + 深红旋风分身池。retail 是**一进本就 spawn 好**(helper 停场中、分身站边缘待机,
-- 都隐藏着),不是开打才刷。所以放在 onSetup(进本即执行),不放 onBattleStart。
-- reset_encounter(灭团)会把池 despawn,然后调 onReset —— 所以 onReset 里再 spawn 一次补回来。
local function spawn_pools(director)
    -- helper 池(≥ 光辉炎柱 8 个同时),停场中;名字用 Boss 的(伊弗利特)。
    director:spawn_omen_pool(12, IFRIT_HELPER_ID, IFRIT_NAME_ID, ARENA_CENTER)
    -- 分身池:一进本就 spawn 在各自的**冲锋点**上、朝向圆心,born hidden(visible=false)。
    -- 用时只 显形 + 播动画 + cast,不再移动 → 没有 lerp,冲刺朝向稳。
    for i = 1, 3 do
        local pos, rot = clone_spot(i)
        director:spawn_clone_pool(1, IFRIT_BASE_ID, IFRIT_NAME_ID, pos, rot)
    end
end

function onSetup(director)
    -- Ifrit 一进本就在场,玩家走近/攻击触发进战(start_battle -> onBattleStart)。
    director:spawn_boss_base(IFRIT_BASE_ID, IFRIT_NAME_ID, IFRIT_HP, IFRIT_LEVEL, IFRIT_SPAWN, IFRIT_FACING)
    -- helper / 分身池也一并在进本时刷好(隐藏待机),与 retail 流程一致。
    spawn_pools(director)
end

-- 玩家与场景互动(这里只需要处理通关后的退出点,避免 onGimmickAccessor 未定义报错 + 卡死)。
function onGimmickAccessor(director, actor_id, id, params)
    if id == EOBJ_EXIT then
        director:abandon_duty(actor_id) -- 离开副本
    end
    director:finish_gimmick(actor_id)
end

function onBattleStart(director)
    print("[ifrit] battle start, alive=" .. #director:alive_players())
    -- 池已在 onSetup(进本时)刷好,这里只开机制循环。
    runCycle(director, 0)
end

-- 一轮机制循环(~48s),结束时把自己排到下一轮。
-- complete_duty / wipe 后引擎会清空 scheduler,这个循环会自动停,脚本不用管。
function runCycle(director, n)
    print("[ifrit] cycle " .. n)

    -- +6s 地火喷发:点名一名玩家,boss 预读条与地面 omen **同时**开始倒计时(并行,不串行)。
    -- 落点 = 点名瞬间玩家脚下(boss 读条期间玩家可走出)。
    director:schedule(6.0, function(d)
        local boss = d:bosses()[1]
        if not boss then return end
        local alive = d:alive_players()
        if #alive == 0 then return end
        local target = alive[math.random(1, #alive)]
        d:cast(boss, ACTION_ERUPTION, 2.2) -- boss 喷发预读条动作(可见 tell)
        d:aoe_circle({                     -- 同时:helper 在玩家脚下出 omen,3s 后落
            source = boss,
            pos = d:position(target),
            radius = 8.0,
            delay = 3.0,
            action = ACTION_ERUPTION_HIT,
            damage = 3000,
            omen = true,
        })
    end)

    -- +14s 烈炎喷射:boss 正面 120° 扇形(r15)。retail 抓包确认 1353 是**瞬发**(只有 CST! 无 CST+,
    -- 不像 1355/1356/1357 那样有读条),所以**不发 cast 包**(不用 omen):直接由结算特效从 boss 身上
    -- 播放喷火动作 + 落伤害(AoeEffect8 sourced from boss → 客户端播 boss 的 1353 动画)。这样不会
    -- 出现"boss 在读条/卡 cast"的样子。
    director:schedule(14.0, function(d)
        local boss = d:bosses()[1]
        if not boss then return end
        -- 瞬发吐火动画(tell):立即从 boss 播 1353 动画,无 cast 条、无伤害。
        -- d:play_action(boss, ACTION_INCINERATE)
        -- retail 抓包:1353 是单个 ActionEffect(无 cast),喷火瞬发、伤害在 ~1.43s 后结算
        -- (AIE+ 29.687 → AIE-/HP/ER 31.121)。这里用 action=0 单独发伤害(数字立即出、不被长动画拖后),
        -- delay 对齐 retail 的 1.43s,正好落在"吐完火"那一刻。
        d:aoe_cone({
            source = boss,
            radius = 15.0,
            angle = 120.0,
            delay = 1.43,
            action = ACTION_INCINERATE,
            damage = 4000,
        })
    end)

    -- +24s 光辉炎柱:retail 有两种 variant,这里按 cycle 奇偶交替放,两种都能跑到:
    --   variant A(场中 4 个):中心附近 ±8 的四个基点圆。
    --   variant B(环形 8 个):半径 20 的环上每 45° 一个圆(parse_boss_timeline 抓包确认)。
    -- 各圆走池里不同 helper(round-robin),与 boss 预读条 **同时** 开始倒计时。
    director:schedule(24.0, function(d)
        local boss = d:bosses()[1]
        if not boss then return end
        d:cast(boss, ACTION_PLUME, 2.2) -- boss 动画

        local spots = {}
        if n % 2 == 0 then
            -- variant A:场中 4 个(±8 十字)
            spots = {
                { x = 8.0, y = 0.0, z = 0.0 },
                { x = -8.0, y = 0.0, z = 0.0 },
                { x = 0.0, y = 0.0, z = 8.0 },
                { x = 0.0, y = 0.0, z = -8.0 },
            }
        else
            -- variant B:半径 20 环上 8 个,每 45°
            local PLUME_RING = 20.0
            for i = 0, 7 do
                local a = i * (math.pi / 4.0)
                spots[#spots + 1] = { x = PLUME_RING * math.sin(a), y = 0.0, z = PLUME_RING * math.cos(a) }
            end
        end

        for _, p in ipairs(spots) do
            d:aoe_circle({
                source = boss,
                pos = p,
                radius = 8.0,
                delay = 3.0,
                action = ACTION_PLUME_HIT,
                damage = 3000,
                omen = true,
            })
        end
    end)

    -- +34s 深红旋风(真机制 v1):主 boss 跳走(隐藏+不可选中)→ N 个分身在边缘环上沿直径
    -- 冲锋(rect 43×12,self-cast 457)→ ~3s 结算 → 主 boss 回场。
    -- v1 用隐形 omen helper 当冲锋者(能看到直线预警+结算,但还看不到可见 Ifrit 分身,后续再补)。
    -- 分身从场地边缘的一个"顶点"起,沿弧边每 30° 一个(1 个顶点 + 其余 0~2 个,先固定 2 个),
    -- 全部朝场中(圆心)冲锋。N 后续可视阶段调整。
    -- 数值据 retail 抓包(parse_boss_timeline,真火神 04_17_57):分身在半径 21 的环上、相邻
    -- 每 45° 一个,boss 先 targetable OFF,~0.7s 后第一个分身读条,之后每 ~2.0s 轮流冲一个(2.7s 读条)。
    local CYCLONE_N            = 3
    local CYCLONE_SPAWN_BUFFER = 0.7            -- 显形到第一个分身读条的缓冲(retail ≈0.73s)
    local CYCLONE_CAST         = 2.7            -- 单个分身读条冲锋时长
    local CYCLONE_STAGGER      = 2.0            -- 分身之间"轮流"冲的间隔(retail ≈2.0s,依次冲)
    local CYCLONE_LINGER       = 2.0            -- 最后一个冲完后的收尾停留
    -- boss 跳走/落地的 ActionTimeline(retail PATE 抓包):先转不可选中→播跳起→到顶点再隐藏;
    -- 回场则先显形→播落地→稍后恢复可选中。
    local CYCLONE_JUMP_UP      = 0x008C
    local CYCLONE_LAND         = 0x008D
    local CYCLONE_JUMP_HIDE    = 0.5

    director:schedule(34.0, function(d)
        local boss = d:bosses()[1]
        if not boss then return end

        -- 本次旋风的随机起始角(retail 冲刺方向是随机的)。reveal 与 charge 都用同一个 base。
        local base = math.random() * 2.0 * math.pi

        -- t0:主 boss 跳走 —— 转不可选中 + 播跳起动画(0x008C),~0.5s 后(顶点)隐藏 + 停 AI。
        d:set_targetable(boss, false)
        d:play_timeline(boss, CYCLONE_JUMP_UP)
        d:schedule(CYCLONE_JUMP_HIDE, function(dh)
            local b = dh:bosses()[1]
            if b then
                dh:set_visible(b, false)
                dh:set_ai_enabled(b, false)
            end
        end)

        -- t0:分身显形 → 播 008D 落地动画 → SetPos 到本次冲锋点+朝向 → 紧接着 force_state_refresh(415)。
        -- retail 就是 SetPos 之后发 415 让客户端**强制刷新**实体的位置/朝向 —— 没有它,move 的旋转在
        -- 客户端不生效(冲刺方向就偏)。有了它,re-position + rotation 才会被客户端真正应用上。
        for i = 1, CYCLONE_N do
            local pos, rot = clone_spot(i, base)
            local c = d:clone(i)
            if c then
                d:set_visible(c, true)
                d:play_timeline(c, CYCLONE_LAND)
                d:move_actor(c, pos, rot)
                d:force_state_refresh(c)
            end
        end

        -- 轮流冲锋:第 i 个分身在缓冲后、每隔 STAGGER 依次读条冲锋。位置/朝向 = 本次随机 base 的冲锋点。
        for i = 1, CYCLONE_N do
            d:schedule(CYCLONE_SPAWN_BUFFER + (i - 1) * CYCLONE_STAGGER, function(di)
                local pos, rot = clone_spot(i, base)
                local c = di:clone(i)
                if not c then return end
                local b = di:bosses()[1]
                -- 冲锋前**不** move:分身已经在 pos、朝 rot(圆心),cast 直接读它当前朝向冲。
                di:cast(c, ACTION_CYCLONE, CYCLONE_CAST, nil, ARENA_CENTER)
                di:aoe_rect({
                    -- 仇恨记到 boss(分身隐身后不该留在仇恨列表);action=0 不另播动画(457 冲刺已由 cast 播)。
                    source = b or c,
                    pos = pos,
                    rotation = rot,
                    length = 43.0,
                    width = 12.0,
                    delay = CYCLONE_CAST,
                    action = 0,
                    damage = 4000,
                })
            end)
        end

        -- 最后一个冲完 + 收尾缓冲后:把分身隐藏(留在原地待命,不移到场外),主 boss 回场恢复。
        local cleanup_at = CYCLONE_SPAWN_BUFFER + (CYCLONE_N - 1) * CYCLONE_STAGGER + CYCLONE_CAST + CYCLONE_LINGER
        d:schedule(cleanup_at, function(dc)
            for i = 1, CYCLONE_N do
                local c = dc:clone(i)
                if c then dc:set_visible(c, false) end
            end
            -- 回场:先显形 + 播落地动画(0x008D)+ 恢复 AI,稍后再转回可选中
            -- (retail RFLG 0 显形与 PATE 008D 落地几乎同时,~0.5s 后才 ATG+)。
            local b = dc:bosses()[1]
            if b then
                dc:set_visible(b, true)
                dc:play_timeline(b, CYCLONE_LAND)
                dc:set_ai_enabled(b, true)
                dc:schedule(CYCLONE_JUMP_HIDE, function(dr)
                    local b2 = dr:bosses()[1]
                    if b2 then dr:set_targetable(b2, true) end
                end)
            end
        end)
    end)

    -- +44s 地狱之火炎:全屏 raidwide(真本是钉子阶段 enrage,这里先当周期全屏)。
    -- boss 亲自施放(可见动画)+ 读条结束即结算 —— 单一 cast,不另发 cast()(避免重复完成包)。
    director:schedule(44.0, function(d)
        local boss = d:bosses()[1]
        if not boss then return end
        d:raidwide({
            source = boss,
            omen = boss,     -- boss 当施法者(可见)
            omen_cast = 2.0, -- 2s 读条
            delay = 2.0,     -- 读条结束落伤害(同一个 effect 完成动画)
            action = ACTION_HELLFIRE,
            damage = 2000,
        })
    end)

    -- +48s 下一轮
    director:schedule(48.0, function(d)
        runCycle(d, n + 1)
    end)
end

function onReset(director)
    print("[ifrit] reset")
    -- reset_encounter 在调用 onReset 前已把 helper/分身池 despawn 了,这里重新刷一份(隐藏待机),
    -- 否则灭团重开后池就空了。重刷的分身是 born hidden,自然回到待机隐藏态。
    spawn_pools(director)
end

function onActorDeath(director, bnpc_id, position)
    -- spawn_boss_base 把 boss 的 layout_id 设成了 base_id,所以这里用 base_id 比对。
    if bnpc_id == IFRIT_BASE_ID then
        print("[ifrit] defeated")
        director:complete_duty() -- 引擎会顺带停掉机制循环
    end
end
