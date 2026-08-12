# Contract v274：宠物维持与冷落判定

状态：已实现。

## 权威规则

- 来源以 RFB `master:src/cmd5.c::calculate_upkeep`、
  `master:src/dungeon.c::process_world_aux_hp_and_sp` 和
  `master:src/melee2.c` 的 neglected-pet 分支为准。
- 职业内容保存原版 `class_t.pets` 除数：High-Mage 为 25，Warrior 与 Archer 为 40。
- 只统计存活且由玩家控制的 actor；普通 `friendly` actor 不参与。普通宠物按怪物等级
  计费，unique 按 `(level + 5) * 10` 计费。
- 免费额度为 `playerLevel * 80 / divisor`。超出 100 后追加一半超额惩罚，最终封顶
  1500；严格大于 `SAFE_UPKEEP_PCT=484` 才显示危险警告。

## 法力与解散

- 维持比例按 `(100 - upkeep) / 100` 缩放既有法力恢复。超过 100 时使用普通等待恢复量
  计算负恢复，避免休息与职业恢复加成反向放大损失；积分资源对非零损失向上取整。
- 世界推进动作均会承担超额维持损失，等待与休息走同一恢复事务。法力降至零且维持仍
  超过 100 时投影 `dismissalRequired`，休息立即中断。
- 新增零时间 `DismissPets`，按稳定实体 ID 顺序解散全部当前受控宠物。当前 UI 没有昵称
  与逐宠物选择表面，因此不复制原版交互循环。

## 冷落判定

- 危险警告在玩家行动开始时锁定，保证本次行动中新召唤的宠物不会立刻背叛。
- 宠物获得自身行动时，先检查低法力门，再严格按原版顺序消费：当前法力豁免、善恶
  立场、1500 维持检定、unique 保留、高等级保留、无父召唤保留，最后判定消失或转敌。
- 当前没有 Politician 永久友好 unique、Monkey Clone 或 monster parent identity 的正式
  消费者；这三类前置特例不预造状态。骑乘、Warlock pact、怪物种族倍率也继续后置。
- Raise Dead、Animate Dead、Enslave Undead 和召唤卷轴无需专用接线：它们既有的
  `controllerId` 或 `summon.ownerId` 自动进入同一规则。

## 协议与基准

- Protocol 1.182，内容包 1.281.0，content hash
  `9b25a4756e96d61660e019ca3aeeacf717949701741091f85500ba74a3428d2a`。
- 无新增存档字段，State Hash Schema 保持 v90。
- 公共玩家投影新增派生维持摘要，21 条 active exact fixture 统一刷新到
  `contract-v274`，零 waiver。
