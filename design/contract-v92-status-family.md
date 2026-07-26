# Contract v92：新状态族（混乱 / 致盲 / 麻痹）

状态：当前 active baseline。协议 1.92，内容包 1.83.0（content hash `3ed414503866baf22dd248b5a6e8bab6836ddfb0b288812a9a4bfd9cbd7eeecc`）；save 容器继续 v1；三个新状态写入既有 `statuses` 权威结构，state hash 沿用 Schema v40。active baseline 共 299 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

FrogComposband 怪物法术中 CONFUSE（223 只）、BLIND（215 只）、PARALYZE（110 只）是位移族之后覆盖面最大的施加型控制。本轮以原创中性机制新增三个状态种类并实现玩家侧行为效果；自由行动（FREE_ACT）、致盲抗性、混乱抗性、怪物侧混乱行为（玩家混乱怪物）与致盲怪物感知留给后续。

新状态种类（引擎常量，状态种类保持开放字符串，无内容白名单）：

- `rfb.status.confusion`（混乱）：移动方向可能被打乱，且完全无法施法/发动技法；
- `rfb.status.blindness`（致盲）：玩家 FOV 压缩到自身所在格；
- `rfb.status.paralysis`（麻痹）：任何推进世界时间的行动被浪费。

状态 tick 语义沿用既有机制：每次标准行动推进 10 世界 tick，状态按世界 tick 衰减（`durationTicks` 80 ≈ 8 次行动）。

## 2. 玩家侧行为与 RNG 边界

- **混乱移动**：Move 结算前先做混乱重定向——`bounded(4)` 抽取为 0（25%）保持原方向且不发事件；否则 `bounded(8)` 按规范方向序（N/NE/E/SE/S/SW/W/NW）选实际方向并发 `status.confused-move`（args: intended/actual）。重定向后的移动照常走可行走/近战/换位逻辑（撞墙即 move.blocked，撞怪即近战）。两次抽取只在状态存在时发生，无混乱时零额外 RNG、回放字节不变。
- **混乱禁施法**：`resolve_player_ability` 最先检查混乱，命中即发 `ability.cast-unavailable`（reason `confused`），零 RNG、不扣资源；世界时间照常推进（v90 拒绝语义）。
- **致盲**：`is_visible` 顶端短路——只有玩家自身格可见。可见性驱动的一切随之变化：visual 回退记忆层、需要可见目标的检定/侦测过滤（transient 侦测条目全部被过滤）、休息不再因"看见敌人"打断（怪物打到身上仍以 damaged 打断）。怪物自身感知不走该 helper，不受影响。零 RNG。
- **麻痹**：分发层在 `advances_world` 判定后把行动替换为内部合成动作 `ParalyzedIdle`（无命令映射）：发 `status.paralyzed`，标准能量消耗，怪物照常行动、状态照常 tick，但不触发等待恢复（等待恢复只属于主动 Wait）。零时间命令（属性提升、召唤指令、退休）与 Rest 不受拦截；休息的每个回合照常 tick 麻痹（躺着不动语义自洽）。

## 3. 协议与事件（1.92）

- `status.confused-move`（args: intended/actual，方向 token 如 `north-east`）；
- `status.paralyzed`（args: status）；
- `ability.cast-unavailable` 新增 reason 值 `confused`（reason 本就是开放字符串，无 DTO 变更）；
- 无新增 outcome DTO；状态本身沿用既有 `StatusDto`。
- Web：两条新消息文案、三个状态名（statusName 映射），无新面板。

## 4. demo 接入（1.83.0）

三种施加能力全部由新增的 `demo.actor.gloom-weaver`（阴霾织者，速度 100）承载——延续 v91 教训：绝不向既有怪物的加权池插条目，历史基线才能零语义漂移。

- `demo.ability.mind-fog`（迷心之雾）：apply-status confusion，80 tick，权重 2；
- `demo.ability.gloom-veil`（阴霾之幕）：apply-status blindness，80 tick，权重 1；
- `demo.ability.numbing-grasp`（麻痹之握）：apply-status paralysis，**20 tick（约 2 次行动）**，权重 1；短时长 + 逆频率冷却（50% → 2 行动）+ 1/4 选择权重共同防止无自由行动机制前的连锁锁死。

三者均为 position/entity 双模、射程 6、需要 LOE，走既有 apply-status 怪物白名单与 v88 目标规划管线，逆频率冷却自动生效。

## 5. 导入器映射（同轮完成）

CONFUSE → `rfb-legacy.ability.confuse`、BLIND → `rfb-legacy.ability.blind`、PARALYZE → `rfb-legacy.ability.paralyze`（均复用 status_ability 模板：entity/position r6 LOE、25 tick）。重跑：casting 怪物 553 → 586，映射 CONFUSE 223 / BLIND 215 / PARALYZE 110，共享能力 84 个，产物过 `inspect-source`。

## 6. 契约场景（v92）

迁移 288 条（零语义漂移，仅哈希更新；三个新门在状态缺席时惰性，实证两轮 288/288 无漂移）后新增 289-299 共 11 条：

- 289 混乱重定向（east→south-east，事件含 intended/actual）；290 混乱保持原方向（25% 分支，无事件）；291 混乱禁施法（reason `confused`，零 RNG）；
- 292 致盲侦测压制（与 215 同布置：明眼检出 [(4,2)]，致盲空检出）；293 致盲休息不因敌可见打断（对照 170：enemy-visible/0 回合 → damaged/1 回合）；
- 294 麻痹浪费回合（原地不动 + 怪物白打一下）；295 麻痹到期后恢复行动（wasted → expired → moved）；299 麻痹下等待不触发资源恢复（对照 167）；
- 296/297/298 阴霾织者分别施放三种能力（双 wait 攒能量 + 种子扫描定选择骰；298 施加麻痹后玩家 move 被浪费，闭环）。

全部场景 saveRoundTrip。织者重定性场景注意 `baseSpeed` 必须写 100（速度校验按新种类比对）。

## 7. 验证

常规全套 + `migrate-baseline` 零语义漂移核对（修 pack 时长后重迁移一次）+ 新场景 `refresh` 后逐条人工审阅；clippy 单独跑并验退出码；桌面 E2E（contentVisualCount 72）。
