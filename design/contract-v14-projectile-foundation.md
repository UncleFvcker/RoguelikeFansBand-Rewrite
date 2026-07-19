# Contract v14：权威 projectile 与发射器基础

状态：协议 1.14 / contract-v14 active baseline

## 已建立边界

- `Fire { direction }` 是消耗标准行动能量的核心命令；没有装备发射器时产生确定性 unavailable 事件。
- 发射器内容声明射程、命中/伤害修正、伤害骰和类型。装备实例进入来源可追踪的远程能力派生。
- projectile 从玩家位置沿八方向逐格推进，在地图边界、阻挡地形或首个实体处停止，不穿透目标。
- 命中复用结构化检定、物理 AC、抗性、伤害和死亡管线。
- 每个 projectile 事件携带结构化 `ProjectileTraceDto`，包含起点、落点和按顺序经过的格子。

## 协议与兼容性

- 协议升级至 1.14；内容包升级至 1.10.0，并新增 `demo.item.resonance-sling`。
- save schema v1 / state hash schema v9 不变；发射器 profile 从内容和装备实例重建。
- contract-v14 从 v13 迁移 49 个 exact fixtures，并新增一条真实发射器命中与轨迹 fixture，共 50 个。

## 后续

下一切片在同一轨迹原语上增加弹药实例事务与投掷落点；当前发射器不消耗弹药。
