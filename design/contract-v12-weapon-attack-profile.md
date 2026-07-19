# Contract v12：武器 AttackProfile 与玩家多段近战

状态：协议 1.12 / contract-v12 active baseline

## 已建立边界

- 内容物品可声明 `meleeProfile`：攻击次数、命中修正、伤害修正、伤害骰和伤害类型；该字段只允许出现在不可堆叠的 `weapon` 槽装备上。
- 装备实例仍是权威来源。存档保存实例与槽位，武器档案从锁定的内容包重建，因此 save schema v1 与 state hash schema v9 不变。
- `DerivedStatsPipeline` 新增近战攻击次数和伤害修正，命中修正进入已有近战能力管线并保留装备实例来源。
- 玩家一次近战行动按稳定顺序逐击执行；每击独立检定和掷伤害，目标死亡后立即停止，不再抽取 RNG 或生成后续事件。
- 恐惧检定仍针对整次行动，只执行一次；失败时不进入多段攻击循环。

## 协议与内容

- 协议升级到 1.12，`PlayerDto` / `EntityDto` 输出 `AttackProfileDto`；旧 `meleeDamage` 暂时保留，避免无关 UI 迁移。
- 原创内容包升级到 1.8.0，新增双击武器 `demo.item.echo-blade`。
- contract-v12 从 v11 迁移 47 个 exact fixtures，并增加武器连续两次命中、第二击击杀后中断的第 48 个 fixture。

## 后续

下一提交继续 contract-v12 的怪物 `MeleeRoutine` / blow 列表。projectile、射击与投掷不进入本阶段。
