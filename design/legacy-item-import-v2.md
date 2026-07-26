# 旧版物品导入 v2（k_info）

状态：已实现（P44，纯工具迭代——协议/契约/演示包零变更）；产物只进 `.local/packs/rfb-legacy/items/`，仓库继续只含原创内容。

## 1. 行格式（按固定 commit init1.c 钉死）

`N:序号或*:英文名`（`&`/`~` 冠词与复数标记剥离后转 kebab id）、`G:字形:颜色`、`I:tval:sval:pval`、`W:等级:extra:等级上限:重量:价格`（重量原生即 0.1 磅，直接入 `weightTenthsPound`）、`P:AC:骰d面:命中:伤害:护甲修正`、`A:深度/稀有度`（多段）、`F:` 竖线旗标多行累积、`D:` 中文描述（不入库，文案继续生成键）。占位条目（N:0）跳过。

## 2. tval → 形态映射

| tval 类 | 形态 |
| --- | --- |
| 20/21/22/23（挖掘/钝器/长柄/剑） | `equipmentSlot: weapon` + meleeProfile（attacks 1、P: 骰与命中/伤害修正；原版攻击次数源于玩家技能，记为差异） |
| 19（弓） | `equipmentSlot: launcher` + projectileProfile（沿用 demo 惯例射程/骰；倍率语义记缺口） |
| 16/17/18（弹/箭/弩矢） | 弹药：无槽、堆叠 99、`ammunition` 标签、破损率 25%；**弹药自带骰折入发射器**记为已知差异 |
| 36/37/38（软甲/硬甲/龙鳞） | `equipmentSlot: body` + `modifiers.defense = AC + to_ac` |
| 34/35（盔/冠） | `head`；33（盾）`shield`；32（披风）`cloak`；31（手套）`gloves`；30（靴）`boots`（同上 defense 映射） |
| 45（戒指） | `ring` 槽；F: 六维旗标（STR/INT/WIS/DEX/CON/CHR）按 pval 映射属性修正；AC 环映射 defense；效果环（速度/隐形等无对应字段）留壳+缺口 |
| 40（护符） | `amulet` 槽，同戒指 |
| 39（光源） | 无槽壳 + `light-source` 标签（demo luminous-shard 先例） |
| 75/80（药水/食物） | 堆叠消耗品壳；效果按 sval 藏于原版代码而非数据，**行为缺口**（治疗类未来以精选表接 use_action Heal） |
| 70/65/55/66（卷轴/魔杖/法杖/权杖） | 壳 + 行为缺口（设备系统未建立） |
| 90+ 魔典族 | 壳 + `book` 标签（旧版法术书系统未映射） |
| 其余（箱子/尖刺/瓶罐/雕像/尸骸等） | 通用壳（identity/重量/堆叠/字形恒可表达） |

堆叠默认按类（装备 1、弹药 99、药水/卷轴/食物 20、其余 10）——原版无堆叠字段，记为近似。

## 3. 缺口报告扩展

`itemsTotal/itemsImported/itemsSkipped`、`unmappedItemFlags`（六维之外的 F: 旗标计数）、`itemBehaviorGaps`（按形态类计数：device-effect / consumable-effect / book-system / ammo-dice-folded / launcher-multiplier / effect-jewelry）。

## 4. 实测

545 条中 544 条导入（跳过占位）；产物过 `rfb-contentc inspect-source` 全部校验（items root 动态加入 pack.json）。数字详见导入报告。

## 5. 遗留

- 药水/卷轴/设备的主动效果等设备与消耗品效果系统扩展后按 sval 精选表接入；
- E:/D: 中文名与描述导出为本地 Fluent 片段（v2 方向未变）；
- ego（e_info）与固定神器（a_info）导入后续排期。
