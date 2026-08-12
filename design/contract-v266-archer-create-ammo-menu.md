# Contract v266：弓箭手“制造弹药”单菜单

状态：已实现。协议 `1.176`，save 容器 `v1`，State Hash Schema `v88`，内容包
`1.261.0`，active baseline `contract-v266`。

## 原版边界

权威 `master:src/archer.c` 只授予弓箭手一个一级职业能力“制造弹药”。该能力内部在
1 级显示制造弹丸、10 级开放制造箭矢、20 级开放制造弩栓。

本实现保留既有三个执行端点：

- `demo.ability.archer-create-shots`
- `demo.ability.archer-create-arrows`
- `demo.ability.archer-create-bolts`

`ClassAbilityDefinition.uiGroupNameKey` 把三个端点投影为同一个“制造弹药”分组。前端只
隐藏尚未达到等级的分组子项；每个已开放子项继续发送既有 `CastAbility`，并沿用方向或
物品目标选择流程。没有新增第四个 ability ID、命令、待处理菜单状态或 Core 分支。

## 兼容边界

`AbilityDto.uiGroupNameKey` 是可选投影字段，因此协议升至 `1.176`。Commit 2 已让玩家
制造弹药的 `damageDiceOverride`、`originKind` 和 `discountPercent` 进入物品存档与状态
哈希；本批将 State Hash Schema 正式升至 `v88`，save 容器仍为 `v1`，不为旧开发存档
提供兼容路径。21 条 active fixture 随公共 State Hash Schema 全量刷新并保持零 waiver。

内容包升至 `1.261.0`，content hash 为
`846d7565a37113590dcee9e2ea187fdbd4ff2786c0fa85fbe61743834ae89d0a`。

## 明确留待共享物品生成

制造弹药的中性 `apply_magic` 路径已经闭合。Good/Bad Luck、Chance virtue、特殊游戏
模式、地下城 good/great 上限和全局 `no_egos` 仍属于共享物品生成上下文，不在
Archer 职业代码中建立替代实现。
