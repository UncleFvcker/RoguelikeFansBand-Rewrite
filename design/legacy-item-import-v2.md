# 旧版物品导入 v2（k_info / e_info / a_info）

状态：已实现并持续回灌（P44 基础物品 + P45 词条与固定神器 + P46 fake bow 修正 + P58–P66 设备/消耗品与卷轴纵切）；legacy 产物只进 `.local/packs/rfb-legacy/`，仓库继续只含原创内容。实例充能、动态设备、恢复物品以及鉴定/侦测/传送/附魔/诅咒卷轴运行时由 Contract v108–v116 定义。

## 1. 行格式（按固定 commit init1.c 钉死）

`N:序号或*:英文名`（`&`/`~` 冠词与复数标记剥离后转 kebab id）、`G:字形:颜色`、`I:tval:sval:pval`、`W:等级:extra:等级上限:重量:价格`（重量原生即 0.1 磅，直接入 `weightTenthsPound`）、`P:AC:骰d面:命中:伤害:护甲修正`、`A:深度/稀有度`（多段）、`F:` 竖线旗标多行累积、`D:` 中文描述（不入库，文案继续生成键）。占位条目（N:0）跳过。

## 2. tval → 形态映射

| tval 类 | 形态 |
| --- | --- |
| 20/21/22/23（挖掘/钝器/长柄/剑） | `equipmentSlot: weapon` + meleeProfile（attacks 1、P: 骰与命中/伤害修正；原版攻击次数源于玩家技能，记为差异） |
| 19（弓） | `equipmentSlot: launcher` + projectileProfile（沿用 demo 惯例射程/骰；倍率语义记缺口）；**无弹药配对的竖琴/长笛/枪械保槽不带射击档**——原版 `obj_is_fake_bow` 语义（占射击槽、不可射击），计 launcher-unpaired（P46 修正，此前误降级为无槽壳） |
| 16/17/18（弹/箭/弩矢） | 弹药：无槽、堆叠 99、`ammunition` 标签、破损率 25%；**弹药自带骰折入发射器**记为已知差异 |
| 36/37/38（软甲/硬甲/龙鳞） | `equipmentSlot: body` + `modifiers.defense = AC + to_ac` |
| 34/35（盔/冠） | `head`；33（盾）`shield`；32（披风）`cloak`；31（手套）`gloves`；30（靴）`boots`（同上 defense 映射） |
| 45（戒指） | `ring` 槽通用壳——**普通首饰无任何属性**（原版属性与 pval 全部由 ego 生成期赋予或固定神器携带，P45 修正）；AC 直接入 defense；效果留壳+缺口 |
| 40（护符） | `amulet` 槽，同戒指 |
| 39（光源） | `light` 槽（contract-v100 身体模板起）+ `light-source` 标签；光源神器六维随槽回收（帕蓝提尔等 8 件）；原版火把可堆叠、半径/燃料语义记差异 |
| 75/80（药水/食物） | 堆叠消耗品；P58 接入六种治疗药水；P61 增加四种状态恢复食物、Boldness、Vigor、Restore Mana、Clarity，并为六种治疗药水补充当前可表达的异常清除序列，其余保留行为缺口 |
| 70/71/65/55/66（卷轴/魔杖/法杖/权杖） | P62 为 sval 12/13 接入鉴定；P63 映射 25–30/57 的地图/侦测；P64 映射 8–11/53 的传送/召回；P65 映射 16/17/18/20/21 的装备附魔；P66 映射 2/3/14/15 的施咒/解除，剩余 38 条计 `scroll-effect`；P59 为通用 wand/staff/rod 壳生成动态候选，效果 identity、power、成本与随机容量在实例生成时物化 |
| 90+ 魔典族 | 壳 + `book` 标签（旧版法术书系统未映射） |
| 其余（箱子/尖刺/瓶罐/雕像/尸骸等） | 通用壳（identity/重量/堆叠/字形恒可表达） |

堆叠默认按类（装备 1、弹药 99、药水/卷轴/食物 20、其余 10）——原版无堆叠字段，记为近似。

## 3. 缺口报告扩展

`itemsTotal/itemsImported/itemsSkipped`、`unmappedItemFlags`（基础物品的全部 F: 旗标——基础件不再映射任何属性）、`egosTotal/egosImported`、`artifactsTotal/artifactsImported`、`unmappedEgoFlags`、`unmappedArtifactFlags`、`itemBehaviorGaps`（按形态类计数：scroll-effect / consumable-effect / book-system / ammo-dice-folded / launcher-multiplier / effect-jewelry / launcher-unpaired / ego-activation / artifact-activation）。`device-effect` 已在 P62 退出报告。

## 4. e_info 词条 → affix（P45）

行格式：`N:序号:名称`（剥 `of ` 前缀转 kebab）、`T:` 竖线槽类多行累积（转小写入 tags）、`W:等级:稀有度:权重`、`C:命中上限:伤害上限:护甲上限:pval 上限`、`F:` 旗标、`E:` 激活。映射：`C:` 为生成期随机上限，取为**确定性顶格值**（已记差异）——attack = max(命中,伤害)、defense = 护甲、六维旗标（含 `DEC_` 负向）按 pval 上限折算；产物为 `rfb-legacy.affix.*`（affix.schema）。**全部力量都在不可表达旗标里的词条**（如 (Arcane) 的 SPELL_POWER/BRAND_MANA）会产出空修正表，被 affix 契约拒绝——按 `ego-inexpressible` 跳过计数，旗标仍进 `unmappedEgoFlags`。激活（E:）计 `ego-activation` 缺口。

## 5. a_info 固定神器 → item（P45）

行格式：`N:序号:名称`、`I:tval:sval:pval`（pval 固定）、`W:等级:稀有度:重量:价格`、`P:AC:骰:命中:伤害:护甲`、`F:` 旗标（`INSTA_ART` 为生成语义、不计缺口）、`E:` ASCII token 视为激活（中文文案行跳过）。映射：复用 k_info 的 tval 形态表，id 为 `rfb-legacy.item.artifact-*`、字形 `*`、maxStack 1、tags 含 `artifact`；武器带固定 meleeProfile（P: 骰与修正），发射器按 sval 配对弹药（竖琴/枪械类 7 件为 **fake bow**：保 launcher 槽与固定修正、不带射击档，P: 命中/伤害为纯射击加成随射击档一并舍弃，计 launcher-unpaired——P46 修正）；AC+护甲修正入 defense，非武器槽位取 max(命中,伤害) 入 attack，六维旗标按固定 pval 折算——**哨兵 pval（混沌盾 125）钳制进契约 ±100 属性窗口**（已记差异）。无槽形态（光源等）不折属性，其六维旗标保留在 `unmappedArtifactFlags` 里不被吞掉。激活计 `artifact-activation` 缺口。

## 6. 实测

k_info 545 条中 544 条导入（跳过占位）；e_info 160 条词条 88 条成为 affix（72 条 ego-inexpressible——力量全在抗性/免疫/速度类旗标）；a_info 392 条神器全数导入。产物（936 items + 88 affixes）过 `rfb-contentc inspect-source` 全部校验（items/affixes root 动态加入 pack.json）。fake bow 修正后 12 件未配对发射器（基础 5 + 神器 7）全部可装备，阿波罗竖琴等取回固定六维（力量/智力/魅力 +5）；契约验证依据：`launcher` 槽不带射击档合法（物品规则均为单向），运行时射击路径查无射击档仅拒绝开火。主要旗标缺口：IGNORE_*/RES_*/SEE_INVIS/FREE_ACT/SPEED/SLAY_* 等待装备旗标系统。

P58 在后续装备旗标/法书回灌结果上重跑真实包：937 items、128 affixes、1260 abilities、4 ability books 全部严格编译，content hash 为 `ed9534de7976be4668a8238deae3d207794d862e7a4ab41e888fde8c7e7b479c`。六种治疗药水退出缺口后，`consumable-effect` 由 95 降至 89；`device-effect` 仍为 64，`artifact-activation` 180、`ego-activation` 13。

P59 为原版通用 wand/staff/rod 壳生成首批动态 activation，并把 `f_info` 的 `TRAP` 旗标映射为 terrain `trap` tag，保证 detect 候选通过严格内容引用校验。真实包仍为 937 items、128 affixes、1260 abilities、4 ability books，content hash 为 `68f8c65c4b80e67437457e1c51ff77b11c2d4a095bb2e9cfa01983c244d427b3`；`device-effect` 64→61，`consumable-effect` 89、`artifact-activation` 180、`ego-activation` 13 保持。

P61 增加有序恢复型物品效果后，四种状态恢复食物、Boldness、Vigor、Restore Mana、Clarity 退出缺口，六种既有治疗药水获得异常清除序列。真实包仍为 937 items、128 affixes、1260 abilities、4 ability books，content hash 为 `b6913ec229580a8decd6816fbebc4af6554bb55cd222fc7e11e9ceec1a353eac`；`consumable-effect` 89→81。复核 `device-effect` 61 后确认全部来自 tval 70/71 卷轴，三种 wand/staff/rod 通用壳已不在该缺口中。

P62 把 tval 70/71 缺口重命名为 `scroll-effect`，并按 sval 12/13 映射普通/完全鉴定。独立 detached worktree 上的真实包保持 937 items、128 affixes、1260 abilities、4 ability books，严格源校验、编译和产物回读 hash 均为 `143ed91ebd453dd22628548663dac0483c28d2f20625b749844a5419c61cac44`；`scroll-effect` 61→59，报告不再含 `device-effect`。

P63 按真实 sval 分布接入地图/侦测七条：25 Mapping、26 Gold、27 Item、28 Trap、29 Door/Stairs、30 Invisible、57 Monsters。导入器同步给 TV_GOLD、f_info DOOR/STAIRS 与 r_info INVISIBLE 增加 `gold`、`passage`、`invisible` 语义标签。真实包保持 937 items、128 affixes、1260 abilities、4 ability books，严格源校验、编译和产物回读 hash 均为 `43b02c9e94aaa8b962d54f3e9b55cf31ab16a3c1a6573e677b2d23df32636abe`；`scroll-effect` 59→52。

P64 接入传送/回城五条：8 Phase Door、9 Teleport、10 Teleport Level、11 Word of Recall、53 Reset Recall。前两者映射为距离 10/100 的 `random-teleport`，Recall 映射原版 `1d21 + 14` 延迟；跨层和目的地重设使用通用楼层/召回事务。真实包保持 937 items、128 affixes、1260 abilities、4 ability books，严格源校验、编译和产物回读 hash 均为 `7d194979fdc047e93f60325f8d3d3b068d75a0f9e0b38eb5be0ecfd0ce77beba`；`scroll-effect` 52→47。

P65 接入五种装备附魔卷轴：16 Armor、17 Weapon To-Hit、18 Weapon To-Dam、20 *Armor*、21 *Weapon*。普通卷轴各作一次尝试，强力卷轴作 `1d3+3` 次；实例强化沿用原版千分递减表、+15 上限、神器 50% 二次门及普通/弹药堆叠门。真实包保持 937 items、128 affixes、1260 abilities、4 ability books，严格源校验、编译和产物回读 hash 均为 `a727f0ef817eefe5d790699da84e88f942a23246b4fd0b4af23b96385649dc57`；`scroll-effect` 47→42。

P66 接入四种装备诅咒卷轴：2 Curse Armor、3 Curse Weapon、14 Remove Curse、15 *Remove Curse*。实例诅咒分 normal/heavy/permanent，施咒具有神器 50% 抵抗；普通解除只移除 normal，强力解除可移除 heavy，永久诅咒保留。真实包保持 937 items、128 affixes、1260 abilities、4 ability books，严格源校验、编译和产物回读 hash 均为 `b517b3dc48395c91b3c9864028cce2f4ae5f97d94dc41264c1afe1ac9af9fb70`；`scroll-effect` 42→38。

P67 接入四种召唤卷轴：4 Summon Monster、5 Summon Undead、6 Summon Pet、54 Summon Kin。Monster/Undead 使用地牢深度并生成敌对结果，Pet 使用地牢深度并永久控制，Kin 使用玩家等级与 Race `kinCategory`；actor/Race 同步导入 glyph 式 category。真实包保持 937 items、128 affixes、1260 abilities、4 ability books，并新增 1332 actor glyph tag 与 68 个 Race kin 映射；严格源校验、编译和产物回读 hash 均为 `fbe1a9682d464e28ade0bd5df8fe8fbdda4fd1030413dd78965a4a4c983834d0`；`scroll-effect` 38→34。

## 7. 遗留

- 其余药水/食物按源码 sval 精选接入；剩余 `scroll-effect` 34 按世界/状态/物品效果重新分组后继续接入；
- 装备旗标系统（抗性/免疫/速度/斩杀支路）落地后重跑导入，可解锁 72 条 ego-inexpressible 词条与神器旗标主体；
- E:/D: 中文名与描述导出为本地 Fluent 片段（v2 方向未变）；
- 词条与基础物品的运行时挂接（生成期 affix 抽取）属于战利品生成线，另行排期。
