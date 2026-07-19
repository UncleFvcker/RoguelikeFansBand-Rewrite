# Contract v13：怪物 MeleeRoutine 与稳定 blow 顺序

状态：协议 1.13 / contract-v13 active baseline

## 已建立边界

- 怪物内容可声明 `meleeRoutine.blows`；每个 blow 包含稳定 method ID、命中修正、伤害骰和伤害类型。
- 每次怪物近战行动严格按内容声明顺序逐 blow 执行，每击独立检定与结算；玩家死亡后立即中断剩余 blow。
- 物理 blow 继续逐击应用 AC，元素 blow 继续逐击应用抗性。命中修正通过来源可追踪的派生属性进入 `CheckContext`。
- 未声明 routine 的旧怪物自动得到一个由既有 `damageDice` / `damageSides` / `damageType` 构造的 fallback blow，旧事件参数保持不变。
- 显式 routine 的命中、失误和致死事件增加 `method` 参数，UI、诊断和未来知识发现无需从怪物类型反推攻击方法。

## 协议、内容与兼容性

- 协议升级到 1.13，`EntityDto` 新增 `MeleeRoutineDto` / `MeleeBlowDto`。
- 原创内容包升级到 1.9.0，新增依次撕咬与抓击的 `demo.actor.echo-hound`。
- save schema v1 与 state hash schema v9 不变；routine 由内容 hash 和既有怪物 kind ID 重建。
- contract-v13 从 v12 迁移 48 个 exact fixtures，并增加锁定两个 blow 顺序、method ID、物理 AC 与冷伤害的第 49 个 fixture。

## 后续

基础玩家/怪物多段近战已经闭环。下一阶段 contract-v14 建立 projectile、射击与投掷；on-hit effect、暴击和品牌在相应攻击阶段接口上继续扩展。
