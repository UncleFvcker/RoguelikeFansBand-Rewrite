# Contract v17：弹药破损与落地回收

状态：协议 1.17 / contract-v17 active baseline

## 已建立边界

- 物品内容定义新增 `breakChancePercent`，取值为 0–100；原创共鸣弹丸使用 10%，对应旧版 shot/bolt 类弹药的基础折损率。
- 射击仍先从最低稳定实例 ID 的背包弹药堆原子取出一件。完整单件保留原实例 ID；从堆叠拆分时使用持久化 `generated.item.N`，即使随后破损也不复用该 ID。
- projectile 没有撞到实体时不抽取破损 RNG，弹药必定落在权威 `landing`；撞到实体时在命中与伤害结算之后进行一次破损检定。
- 未破损弹药成为 `landing` 上的地面实例，并产生 `combat.projectile-ammo-recovered`；破损弹药被销毁，并产生 `combat.projectile-ammo-broken`。
- 可见格、地面物品、事件、RNG 和实例分配器均进入现有快照、存档、回放与 state hash 闭环，不新增存档或 state hash schema。

## 版本与基线

- 协议升至 1.17，用于标记新增事件语义和确定性 RNG 顺序；DTO 结构没有新增字段。
- 原创内容包升至 1.12.0；1.11.0 content hash 加入显式内置存档迁移白名单。
- contract-v17 从 v16 迁移 53 个 exact fixtures，并新增一条确定性破损 fixture，共 54 个。

## 后续

下一规则切片建立投掷攻击 profile、重量射程与命中/伤害；特殊返回弹药、职业折损修正和投射物动画留待相应系统建立后扩展。
