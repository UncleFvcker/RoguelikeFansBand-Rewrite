# Contract v15：弹药事务与投掷落点

状态：协议 1.15 / contract-v15 active baseline

## 已建立边界

- 发射器 `projectileProfile` 使用稳定 `ammoKindId` 引用可堆叠且带 `ammunition` tag 的弹药内容。
- `Fire` 在轨迹和命中检定前，从背包中按实例 ID 稳定选择弹药堆并消耗一件；没有匹配弹药时产生确定性 unavailable 事件，不推进 RNG。
- `ProjectileTraceDto` 将碰撞 `impact` 与实际可放置的 `landing` 分开；墙壁或边界碰撞时，落点保持在最后一个可行走格。
- `Throw { itemId, direction }` 当前使用固定 4 格射程，每次只移动背包中的一件物品。数量大于一时使用持久化 `generated.item.N` 拆分新实例，原实例保留剩余数量。
- 投掷与射击共用权威直线轨迹并在首个实体、阻挡地形或边界停止；本阶段只建立物品落点，不结算投掷命中或伤害。

## 协议与兼容性

- 协议升级至 1.15；内容包升级至 1.11.0，并新增 `demo.item.resonance-pellet`。
- save schema v1 / state hash schema v9 不变；弹药消费与投掷实例位置已经由现有物品状态覆盖。
- contract-v15 从 v14 迁移 50 个 exact fixtures，并新增弹药消费和堆叠物投掷两条 fixture，共 52 个。

## 后续

下一切片建立目标选择协议，并为弹药破损/回收、投掷命中与伤害保留明确阶段；当前发射后的弹药直接消耗，投掷物则始终落地。
