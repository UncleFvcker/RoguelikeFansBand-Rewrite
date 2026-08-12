# Contract v284：挖矿额外物品品质与固定神器

状态：已实现

## 规则

- 富矿额外物品继续使用原版概率 `min(20, 3 + depth / 15 + mining / 1000)%`。
- 命中额外物品后，用同一个 `0..99` 骰按顺序划分品质：
  - Artifact：`scaled(mining, 5)`；
  - Great：`scaled(mining, 20)`；
  - Good：`scaled(mining, 40)`；
  - 其余 Ordinary。
- `scaled` 使用原版 `(maximum * clamp(mining, 0, 8000) + 4000) / 8000`。
- Artifact 档最多调用 20 次共享 Artifact 生成请求，只接受带正式
  `artifactGeneration` 的固定神器；全部失败后只调用一次 Great 请求。
- 被丢弃的草稿不分配实例 ID、不登记唯一神器状态。最终采用的物品在提交时分配实例 ID，
  并统一记录 `originKind = rubble`。
- 魔法破墙不进入玩家挖矿收益路径，不增加熟练度或材料，也不尝试额外物品与神器。

## 版本与验证

- Protocol：`1.187`（不变）。
- State Hash Schema：`v93`（不变）。
- save：`v1`（不变）。
- pack：`1.294.0`（不变；本批无内容 ID 或内容数据变更）。
- active baseline：`contract-v284`，26 条 exact fixture，零 waiver。
- 现有 active fixture 未进入额外物品品质分支，因此不刷新快照；新增核心测试覆盖概率边界、
  第 20 次成功、20 次失败后的唯一 Great 回退、固定神器唯一性、实例序号、状态哈希，以及
  岩浆/石英的显式与隐藏富矿和魔法破墙隔离。
