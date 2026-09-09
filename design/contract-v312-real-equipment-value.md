# E8.1：生成上下文与真实装备估值

日期：2026-09-09。工作树 `RoguelikeFansBand-Rewrite-realms-items`，分支 `codex/realms-items`。
权威来源为 `D:/codex/Frogcomposband/master` 的 Git `master` 对象，解析为
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。

本批对应[共享生成计划](ego-shared-generation-plan.md)的 E8.1。负向质量抽样、价值驱动诅咒、
随机神器和首饰筛选分别由 E8.2、E8.5、E8.6 启用；本批提供其真实估值基础。

## 实例表示与估值

[item_value.rs](../crates/rfb-core/src/game/item_value.rs)移植 `object3.c` 的七类 COST_REAL
公式及共享能力、抗性、属性、slay/brand、激活和诅咒计算，分派对应 `object2.c:obj_value_real`。
[实例读取](../crates/rfb-core/src/game/item_value/instance.rs)直接读取实际底材、完整属性与实例状态。
无 RNG、无鉴定副作用、不读取玩家已知信息；不把该数值投影到 UI，也不改 `town.rs` 的基础定价。
装置继续属于独立 device value 契约；不支持的类型或缺少权威元数据的物品返回 `None`，没有近似价格兜底。

- `ItemDefinition.rfbValue` 保留底材/固定神器的原始 flags、pval 及基础 AC 与 to_a 的分界。
  `rfbEgo.flags` 保留 Ego 固有标记，包括 pval 为零时仍存在的标记。
- 生成属性的 `rfbPval` 保存原版共享 pval 和标记；`rfbFlags` 保存游戏投影会合并的区别，
  包括 OF 与 OFC 诅咒、同时存在的 slay/kill、抗性/弱点/免疫、光明/黑暗。
  两项均进入存档和状态哈希；读档不重新生成。炸毁清除动态 flags，但保留源码没有清除的 pval。
- 激活档案的 `rfbValue` 是固定 effect power 下、冷却修正前的原版 EFFECT_VALUE。
  装备初始化使用 effect 自身等级，生成深度只参与候选选择；装置仍使用独立深度规则。
  估值再应用实际恢复间隔；鉴定、剩余充能和冷却进度不改变 COST_REAL。
- 固定神器光源走首饰分支，普通光源与箭袋保留源码重复计入 stats 的顺序。
  固定神器保留普通底材 flags；空燃料光源按 `obj_flags` 排除相应 Ego 固有标记。
- 原有 Paurnimmen 导入把命中/伤害合并成 attack。现在分别保留并消费命中 +2、伤害 +2；
  Arkenstone 基础身份补为 k_info 625，供其固定神器读取原始底材。

负 pval -1/-2 会使原版 unsigned flag score 超出 signed 转换范围，C 标准对此没有可移植保证。
参考程序使用 x86-64 GCC、`-O0 -fwrapv`，Rust 明确保留该环境的整数转换结果与 32 位加法行为；
不声称这组越界输入在所有 C 编译器/优化级别下有相同结果。普通范围仍按源码截断、分段、乘算、
相反标记消解及最终归零顺序逐项对照。

## 真实生成入口

生成期使用有符号 power；显示质量只在最终 draft 中投影，不保存 power。
`LootContext.depth` 表示 source `object_level`，`floor_id` 单独定位实际 dungeon depth；
固定神器的越深检查保持两者各自的用途。坏运气调整、Chance 美德从当前真实玩家状态读取。

| 入口 | 模式与等级来源 | 本批处理/范围 |
| --- | --- | --- |
| 自然楼层、房间、vault、普通物品生成 | Ordinary；调用点 object_level | 正向 0/1/2；条件掷骰顺序不从显示品质反推 |
| 怪物死亡 | DROP_GOOD、DROP_GREAT；`_mon_drop_lvl` 的怪物/楼层等级 | 物品与金币都使用计算出的 object_level，不能只把该等级用于金币 |
| 怪物主题掉落 | 保留本次主题骰的实际结果 | 首饰 power 为零时升为 1；基础主题分配表的完整重建仍是独立任务 |
| 只有 DROP_GREAT 的怪物 | `GreatOnly` 对应 AM_GREAT、不含 AM_GOOD | 仍抽外层 good 骰，内层 great 短路；6 条 source 定义保留 `greatOnly` |
| 强制 Good / Great | AM_GOOD / AM_GOOD\|AM_GREAT | 分别保留内层骰 / 两层都短路 |
| 挖掘特殊结果、指定神器奖励的回退请求 | AM_GOOD\|AM_GREAT\|AM_SPECIAL | 显式 power 3、4 次固定神器尝试；随机神器调度留 E8.5 |
| 已指定的固定神器 | 独立固定实例创建 | 不参与 Ego 抽选；静态激活使用其 effect 等级 |
| 固定任务奖励、出生物品和地图显式物品 | 已配置 kind/affix；各调用点自己的物化等级 | 直接物化，等级不能一律替换成当前楼层；显式传入 power 2，不再由显示质量反推；源端奖励模式差异见下文 |
| Craft 卷轴 | 现有工艺事务，玩家等级 | 原有候选选择/提交边界保持；不新增固定神器或随机神器抽样 |

`GreatOnly` 六条为 source 1214、1215、1227、1230、1266、1303。`AM_GREAT` 单独存在时
不蕴含 `AM_GOOD`；瞬时神器机会仍为 1/1000。源码 AM_FORCE_EGO 会排除固定神器并按类型/等级
调整 power，AM_QUEST 会先给 apply_magic 等级加 10；两者不能把通用显示质量当作等价表达。
源端具体映射另行核对为：

- `rooms.c:2015–2023` 的指定 kind + Ego 使用 GOOD|GREAT|FORCE_EGO；指定随机 Ego 使用
  GOOD|GREAT|NO_FIXED_ART；指定随机神器使用 GOOD|GREAT|SPECIAL|NO_FIXED_ART。
  当前地图显式 kind/affix 采用直接配置式物化，不执行这套完整 apply_magic 调度。
- `quest.c:1385–1390` 的守卫 final_object + final_ego 使用 NO_FIXED_ART|GOOD|FORCE_EGO，
  无 Ego 时为 NO_FIXED_ART|GOOD|QUEST；当前守卫奖励表保持配置式路径，不能把 Ordinary 表或
  固定神器回退的 Artifact 请求宣称为完整源端奖励模式。源端替换神器的 create_replacement_art
  属于 E8.5；龙甲奖励的基础生成属于 E8.3。
- `quest.c:221/522/2357` 的程序式 Angband 任务奖励为 GOOD|GREAT|TAILORED|QUEST；
  当前固定任务奖励表没有该程序式奖励入口。其 +10 等级和 tailored 条件不能套在所有配置式奖励上。
- `ego.c:3992` 的 Craft 最终物化使用 power 2、CRAFTING；现有 Craft 事务直接传递玩家等级并
  跳过底材重建、固定神器与随机神器候选抽样。

上述尚无完整调度入口的模式不以恒假开关表示“已接入”。AM_AVERAGE、AM_CURSED 也未开放；
本批记录参数来源与缺口，后续 E8.2/E8.3/E8.5/E8.6 在对应生成消费者中接入，E8.8 复核奖励及
显式创建的完整模式等价性。E8.1 的真实估值和 signed power 不依赖这些未开放的调度分支。

## 职业、种族及条件分支清单

该清单以源端实际条件分类；未开放构筑不伪造身份作为验收，也不为它们加入未使用参数。

| 条件 | 源码与作用 | 当前归属 |
| --- | --- | --- |
| Mauler | ego.c:1641 重武器重量；object2.c:2466 tailored 武器限制；artifact.c:2227 bias | 真实职业未开放，E8.7 |
| Bard | object2.c:2245/2298 竖琴 pval；artifact.c:2044 槽位比例、2286 bias | 当前保留非 Bard 路径；Bard 随职业接入 |
| Monster Ring | ego.c:455 激活概率；object2.c:2498/3072 首饰选择 | 真实种族未开放，E8.7 |
| Berserker | artifact.c:1323 anti-teleport 权重、2218 bias | 随随机神器及职业接入 |
| 已开放 Warrior、Archer、Cavalry | artifact.c:2217–2221 Warrior bias | E8.5 的真实 create_artifact 消费者 |
| 已开放 High Mage、Sniper、Paladin | artifact.c:2231/2257/2261 Mage/Ranger/Priestly bias 和比例 | E8.5；不能用目前 Ego 测试代替 |
| 其他职业 bias | artifact.c:2215–2329 完整 switch，包括 Disciple 子类、默认分支 | E8.5/E8.7，按真实职业开放逐项接入 |
| Archer、施法职业、轻甲职业及近战限制 | object2.c:2418–2508 的 ammo/gloves/weapon/armor suitability | tailored 基础分配，非本批七类 COST_REAL |
| Tomte、Monster Mummy | object2.c:2493/2508 帽子与光源适用性 | tailored 装备过滤，随实际入口处理 |
| Rage Mage 及多领域职业 | object2.c:2639/2745/3444–3465 书籍与领域分配 | 书籍/职业任务，非 Ego 估值 |
| Archer/Sniper、Cavalry/Beastmaster、Device 专职及 Monster 各形态 | object2.c:3564–3640 tailored 基础种类选择 | 完整基础分配/真实职业种族入口 |
| Vortex | 本次三个源文件未发现直接以该名称/常量判断的生成分支 | 更正旧审计的笼统说法，待实际装备模板/消费者链核查 |
| Luck、Chance、主题 | object2.c:2044 后的真实当前玩家/掉落状态 | 本批继续使用真实玩家数据，主题骰结果随上下文传递 |

object2.c 中 Politician 的金币回调、后段自动拾取的职业判断不属于装备生成；不把文本命中都算成
Ego 缺口。当前没有因此增加职业配置、显示名称或 UI 开关。

## 复核与版本

```powershell
python scripts/generate-item-value-reference.py D:/codex/Frogcomposband/master --sync
cargo test -p rfb-core game::item_value
cargo test -p rfb-core all_160_source_egos_have_an_effect_and_save_stable_instances
```

脚本只读取 Git 对象，编译原始 object3.c 及所需源端 helper；295 组公式向量的期望值来自 C。
另有 146 个底材/固定神器输入直接从 k_info/a_info 提取并交给 C 求值，验证实际实例接入。
新增/重导入装备激活后运行 `--sync`，再更新 pack version/content lock；不手写近似激活价值。

版本为 Protocol 1.235、State Hash Schema v112、save header/payload v7（容器 v1）、pack 1.392.0。
新增实例字段及共享初始化/生成上下文变化要求刷新并复验全部 active 契约；不修改场景或放松断言。
已刷新并复验 contract-v312 的 26 条 active fixture，零 waiver；逐文件核对除 assertions 外的
场景、前置条件和命令均未变化。最终验证：

- rfb-core lib：914 项通过、1 项按原设置忽略；随后新增/调整代码复验 item_value 6 项、tasks 37 项通过。
- rfb-content 133、rfb-legacy-import 174、rfb-protocol 7、rfb-save 2 项库测试通过。
- rfb-contract、rfb-replay、rfb-localization 的库与集成测试共 64 项通过；完整重放另由 verify-all 覆盖。
- 前端 181 项通过；TypeScript typecheck 与 Vite UI build 通过（已有大 chunk 提示仍在）。
- workspace（排除 Tauri）all-targets Clippy -D warnings、格式、内容锁、协议绑定和内容 schema 检查通过；
  最后一处生成模式修正后再次通过 rfb-core all-targets Clippy。
- 160 项 source Ego 审计、138 个底材身份及 41 类导入标记检查通过；runtimeParityComplete 仍为 false。

普通商店价格、未知信息展示和桌面打包不属于本批变更。
