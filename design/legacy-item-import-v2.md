# 旧版物品导入 v2（k_info / e_info / a_info）

状态：已实现（P44 基础物品 + P45 词条与固定神器，均为纯工具迭代——协议/契约/演示包零变更）；产物只进 `.local/packs/rfb-legacy/{items,affixes}/`，仓库继续只含原创内容。

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
| 45（戒指） | `ring` 槽通用壳——**普通首饰无任何属性**（原版属性与 pval 全部由 ego 生成期赋予或固定神器携带，P45 修正）；AC 直接入 defense；效果留壳+缺口 |
| 40（护符） | `amulet` 槽，同戒指 |
| 39（光源） | 无槽壳 + `light-source` 标签（demo luminous-shard 先例） |
| 75/80（药水/食物） | 堆叠消耗品壳；效果按 sval 藏于原版代码而非数据，**行为缺口**（治疗类未来以精选表接 use_action Heal） |
| 70/65/55/66（卷轴/魔杖/法杖/权杖） | 壳 + 行为缺口（设备系统未建立） |
| 90+ 魔典族 | 壳 + `book` 标签（旧版法术书系统未映射） |
| 其余（箱子/尖刺/瓶罐/雕像/尸骸等） | 通用壳（identity/重量/堆叠/字形恒可表达） |

堆叠默认按类（装备 1、弹药 99、药水/卷轴/食物 20、其余 10）——原版无堆叠字段，记为近似。

## 3. 缺口报告扩展

`itemsTotal/itemsImported/itemsSkipped`、`unmappedItemFlags`（基础物品的全部 F: 旗标——基础件不再映射任何属性）、`egosTotal/egosImported`、`artifactsTotal/artifactsImported`、`unmappedEgoFlags`、`unmappedArtifactFlags`、`itemBehaviorGaps`（按形态类计数：device-effect / consumable-effect / book-system / ammo-dice-folded / launcher-multiplier / effect-jewelry / launcher-unpaired / ego-activation / artifact-activation）。

## 4. e_info 词条 → affix（P45）

行格式：`N:序号:名称`（剥 `of ` 前缀转 kebab）、`T:` 竖线槽类多行累积（转小写入 tags）、`W:等级:稀有度:权重`、`C:命中上限:伤害上限:护甲上限:pval 上限`、`F:` 旗标、`E:` 激活。映射：`C:` 为生成期随机上限，取为**确定性顶格值**（已记差异）——attack = max(命中,伤害)、defense = 护甲、六维旗标（含 `DEC_` 负向）按 pval 上限折算；产物为 `rfb-legacy.affix.*`（affix.schema）。**全部力量都在不可表达旗标里的词条**（如 (Arcane) 的 SPELL_POWER/BRAND_MANA）会产出空修正表，被 affix 契约拒绝——按 `ego-inexpressible` 跳过计数，旗标仍进 `unmappedEgoFlags`。激活（E:）计 `ego-activation` 缺口。

## 5. a_info 固定神器 → item（P45）

行格式：`N:序号:名称`、`I:tval:sval:pval`（pval 固定）、`W:等级:稀有度:重量:价格`、`P:AC:骰:命中:伤害:护甲`、`F:` 旗标（`INSTA_ART` 为生成语义、不计缺口）、`E:` ASCII token 视为激活（中文文案行跳过）。映射：复用 k_info 的 tval 形态表，id 为 `rfb-legacy.item.artifact-*`、字形 `*`、maxStack 1、tags 含 `artifact`；武器带固定 meleeProfile（P: 骰与修正），发射器按 sval 配对弹药（竖琴/枪械类 7 件同基础件降级为壳并计 launcher-unpaired）；AC+护甲修正入 defense，非武器槽位取 max(命中,伤害) 入 attack，六维旗标按固定 pval 折算——**哨兵 pval（混沌盾 125）钳制进契约 ±100 属性窗口**（已记差异）。无槽形态（光源等）不折属性，其六维旗标保留在 `unmappedArtifactFlags` 里不被吞掉。激活计 `artifact-activation` 缺口。

## 6. 实测

k_info 545 条中 544 条导入（跳过占位）；e_info 160 条词条 88 条成为 affix（72 条 ego-inexpressible——力量全在抗性/免疫/速度类旗标）；a_info 392 条神器全数导入。产物（936 items + 88 affixes）过 `rfb-contentc inspect-source` 全部校验（items/affixes root 动态加入 pack.json）。主要旗标缺口：IGNORE_*/RES_*/SEE_INVIS/FREE_ACT/SPEED/SLAY_* 等待装备旗标系统。

## 7. 遗留

- 药水/卷轴/设备的主动效果等设备与消耗品效果系统扩展后按 sval 精选表接入；
- 装备旗标系统（抗性/免疫/速度/斩杀支路）落地后重跑导入，可解锁 72 条 ego-inexpressible 词条与神器旗标主体；
- E:/D: 中文名与描述导出为本地 Fluent 片段（v2 方向未变）；
- 词条与基础物品的运行时挂接（生成期 affix 抽取）属于战利品生成线，另行排期。
