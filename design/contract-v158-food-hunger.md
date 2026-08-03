<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v158: Food and hunger

状态：historical baseline；Phase 18 Gate 2 完成。协议升至 `1.125`，save 容器保持 v1，state hash Schema 升至 `57`。demo 内容包升至 `1.148.0`，content hash 为 `3f113d088273af9ac1098e78f5a6c8bd597f043a90f8dfc48bc21af9c316aabf`；该基线包含 458 条 exact fixtures、零 waiver。

## 固定来源结论

固定 RFB v1.3.0.7 来源中，普通角色出生饱食度为 `9999`，Warrior 获得 `5-9` 份口粮。Ration of Food 提供 5000 nutrition、重 1.0 磅。正常消化每 50 个能量脉冲结算一次；Bloated 会更快消化，低饱食度依次降低自然恢复、触发昏厥和造成挨饿伤害。

## 实现

- 玩家权威状态新增 `nutrition: u16`，合法范围 `0..=15000`，并投影 Bloated、Full、Normal、Hungry、Weak、Faint 和 Starving 七档状态。
- 生产 Warrior 在出生金币之后抽取一份 `5-9` 数量的口粮堆；历史测试 build 不获得口粮，也不增加出生 RNG。
- Ration of Food 通过既有 `UseItem` 事务消耗一份，增加 5000 nutrition 并封顶 15000，随后支付标准 100 能量行动成本。进食和状态跨阈值使用结构化事件。
- 世界处理每 10 ticks 检查饥饿规则。Bloated 每次扣 100；其他状态仅在 `world_tick % 50 == 0` 时按当前速度的调度能量增益消化。
- nutrition 低于 1000 时自然 HP 恢复降低；低于 500 且未麻痹时每次世界处理有 10% 昏厥并施加 `1d4` paralysis；低于 100 时先造成 `(100 - nutrition) / 10` 物理伤害，死亡会中断后续世界阶段。
- Warrens 深度 1-9 每层在普通物品分配后执行一次 50% 口粮保证尝试；落点可行走、由核心 RNG 决定并随楼层保存。
- Web 玩家栏以百分比显示 nutrition，例如 `15000` 显示为 `150%`，同时显示本地化饱食状态。

## 迁移与确定性

- save 容器仍为 v1。旧 save 缺少 nutrition 时确定性迁移为 `9999`；不补发口粮，不重放出生或楼层生成，也不改变 revision、turn、world tick 或 RNG。
- world tick 顺序保持持续伤害、饥饿/挨饿、自然恢复、设备与怪物阶段。等待和休息使用同一时钟，不按 UI 命令数量单独扣除 nutrition。
- state hash Schema 57 纳入 nutrition。协议 1.125 的快照、存档、TypeScript bindings 与 JSON Schema 同步更新。

## 暂缓边界

- 本 Gate 只加入 Ration of Food，不批量导入蘑菇、酒、特殊食物或特殊种族消化规则。
- 不加入商店库存、购买或出售；补给消费端先独立成立。
- 快速/缓慢消化、德行、特殊姿态和其他饥饿修正继续暂缓。

## 验收

- 核心测试覆盖 Warrior 出生数量和 RNG 顺序、进食封顶、速度相关消化、等待/休息共用时钟、恢复倍率、昏厥、挨饿死亡、存档迁移和 Warrens 楼层口粮。
- fixture 458 固定 seed 42 Warrior 使用出生口粮后 nutrition `14999`、world tick `10`、完整状态和 save round-trip；全部 458 条 active fixtures 保持 exact，零 waiver。
- 内容源码验证、协议/schema 生成检查、Rust workspace、contract、Web 测试、类型检查与 UI 构建共同构成 Gate 2 验收。
