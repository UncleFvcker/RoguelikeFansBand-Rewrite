# E8.2：自然负向装备与价值驱动诅咒

日期：2026-09-09。工作树 `RoguelikeFansBand-Rewrite-realms-items`，分支 `codex/realms-items`。
权威来源为 `D:/codex/Frogcomposband/master` 的 Git `master` 对象：
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，源码读取均通过 Git 对象完成。

对应[共享生成计划](ego-shared-generation-plan.md)的 E8.2。普通装备和 Ego 的负向调度、
诅咒属性、现有角色系统中的消费者、鉴定和存档已经接通；负向随机神器留在 E8.5。

## 生成顺序与类型差异

`object2.c:apply_magic` 的 good 失败后才抽负向质量，再按 no_egos、great 概率和深度处理
`-1/-2`。戒指、项链和装置跳过普通负品质的深层回退。强制 Good/Great/Special 保留源码的
短路顺序；GreatOnly 仍先抽 good。`WorldDefinition.noEgos` 提供实际世界规则入口，正式世界
默认关闭该选项；首饰保留源码例外，没有添加新的出生设置界面。

特别保留两项源码行为：`magik(P)` 在 `P <= 0` 时为真且不抽 RNG；`randint0/1` 的上限为
0 或 1 时不抽 RNG。概率不提前裁剪成通常意义上的 0–100%。

| 类型 | power -1 | power -2 |
| --- | --- | --- |
| 武器、发射器、挖掘工具 | 只减第一组附魔，实际命中与伤害总和为负时附普通诅咒 | 正向两组附魔，物化 Ego 后最终诅咒 |
| 护甲 | 负第一组 AC 附魔，实际 to_a 为负时附普通诅咒 | 正向两组 AC 附魔、Ego、最终诅咒 |
| 弹药 | 原版负附魔分支 | 正向附魔，负 power 不选择 Ego |
| 箭袋 | 基础容量 | 基础容量，不选择 Ego，不统一附诅咒 |
| 火把、提灯 | 燃料初始化后返回 | 光源 Ego 与最终诅咒 |
| 戒指、项链 | 类型内修改局部 power，结束时 -1 转 -2 | 按分支结束后的局部 power 决定诅咒 |
| 装置 | 现有装置初始化 | abs(power) >= 2 选择装置 Ego；原版装置诅咒代码被注释，不统一诅咒 |

保护戒指在循环内继续递减 power，不能根据外层符号决定最终诅咒。普通项链加入正式基础
掉落表，按 `k_info` 的 `A:10/1` 使用深度 10、权重 100。火把、提灯的原始 pval 是燃料，
`obj_create_lite` 将其移入 xtra4 后清零；估值读取同步处理，避免把 4000 燃料当作属性 pval。

## 属性、诅咒与消费者

[curses.rs](../crates/rfb-core/src/game/ego/curses.rs)对应 `artifact.c` 的 `_add_bad_flag`、
`one_biff`、`curse_object` 和 `mspells1.c:get_curse`。通用附魔与 Ego 物化后，先读取 E8.1
完整真实估值，计算 `value / 10000`，再执行次数、嵌套骰、severity 与属性抽样，最后补齐 pval。

- 相反正属性存在时拒绝负属性，不删除标记；重复负标记仍算成功。只有免疫、没有抗性时，
  保留源码允许添加弱点的行为。
- 28 位 get_curse 表排除两个 FLAGGY 位，生成 26 种 CF。LOW_MELEE 限 tval 19–23，
  LOW_AC 限 30–38；武器判定不包含弹药。
- 独立 `rfbHeavyCurse` 位保留 PERMA 与 HEAVY 的区别，PERMA 不自动获得重诅咒罚值。
- 解咒清除 CF 和 HEAVY，移除空 rolled 记录；保留负属性及 NO_TELE 等固有 OF 标记。

周期效果接在现有本地楼层每 10 tick 路径，顺序对应 `dungeon.c`；单候选选择不抽 RNG。
下表使用协议枚举标识，中文显示标签采用权威 `obj_info.c` 中文字符串。

| CF 效果 | 实际消费者 |
| --- | --- |
| Aggravate | 唤醒怪物、激怒感知，沿用 Fairy 例外 |
| Teleport | 1/200，先触发骰再检查反传送，距离 40 |
| TyCurse | 1/200，已有远古邪恶诅咒解析 |
| ByCurse | 1/200，恐惧/眩晕/属性损失/幻觉/经验与装备衰减，再永久降低随机属性 |
| Normality | 1/128，1d20 次清除；每次最多 200 次抽取 33 个原版分支 |
| Allergy | 未不适时 1/888，种族条件通过后添加 70 tick 不适 |
| CrappyMutation | 1/1500，从真实 Bad/Awful 候选获得变异 |
| DrainExperience | 非 Android 时 1/4，当前与最大经验减 ceil(level/2) |
| AddLightCurse、AddHeavyCurse | 各 1/2000，选择实际装备并执行 get_curse(0/1) |
| CallAnimal、CallDemon、CallDragon | 1/2500、1/1111、1/800，实际敌对群体召唤，可选唯一怪 |
| Cowardice | 1/1500，经威胁等级和 CHR 豁免后添加 50 tick 恐惧 |
| DrainHp | 1/666，损失 min(level×2,100) HP |
| DrainMana | 施法资源池非零时 1/666，损失 min(level,50) |
| DrainPack | 1/333，十次背包选择；杖/魔杖减 100、棒减 33，守护生命与免吸充能生效 |
| LowMelee | 当前武器/发射器命中 -5，HEAVY 时 -15 |
| LowArmor | 当前装备 AC -10，HEAVY 时 -30 |
| LowMagic | 每件施法失败率 +3，HEAVY 时 +10，可叠加 |
| LowDevice | 装置技能 -5，HEAVY 时 -10，多件取最大罚值 |
| Catlike | 全局潜行 -4，不按件叠加 |
| SlowRegeneration | 自然 HP 恢复降至五分之一，装备回血周期变为五倍 |
| FastDigest | 食物消耗在慢消化之前 +30 |
| OpenWounds | 当前伤口倒计时的自然恢复频率降至三分之一，伤口伤害不变 |
| Danger | 怪物分配及召唤等级提升，地表折减；影响已有 Cult of Personality 检查 |

CF 仅在装备且仍被诅咒时生效。AGGRAVATE、TELEPORT、TY_CURSE、DRAIN_EXP 的 OF 标记
读取实际底材、Ego 和实例，解咒后仍可生效。未诅咒固有 TELEPORT 保留重写版自动传送、
距离 50 和铭文 `.` 抑制，没有新增原版询问界面。

Allergy 源码 `(!race->flags & RACE_IS_NONLIVING)` 对完整 flags 取逻辑非；不能改为普通的
“不是非生命种族”。当前种族依权威 flags 排除，测试验证 Human 排除和 Hobbit 实际触发。
Normality 保留全部抽样位置，当前没有对应状态/职业机制的分支消耗骰后继续重试，不声明这些
未开放机制已实现。OpenWounds 适配现有倒计时模型，没有新增原版 cut 数值系统；伤害、召唤
和状态行为仍受各自现有契约约束。

## 鉴定、保存与验证

未鉴定动态固有属性和附魔从背包、地面及战斗预览隐藏；真实装备效果仍消费完整属性。
保留当前“装备即完整鉴定”规则，闭环测试在装备前核对隐藏、装备后核对鉴定。

Protocol 1.236；save header/payload v8、容器 v1；State Hash Schema v113；包 1.393.0。
Ordinary 显示品质允许正式 RFB Ego 实例，活动物品、存储楼层和存档使用同一验证规则。
恢复不抽 RNG、不重新生成，不提供旧开发存档兼容。

- 独立 C 脚本抽取权威函数，使用同一注入随机流验证 1,216 组质量、诅咒、get_curse 输入，
  比较结果、draw count 和四字 RNG 终态；不声称重写版采用原版随机数发生器。
- 全部 26 种 CF 的生效/鉴定/解咒边界及具体消费者测试，包含正常/HEAVY/PERMA 罚值、
  空资源池、种族正反条件、设备免吸充能、同 tick 新增诅咒和真实召唤结果。
- 武器、护甲、光源、戒指、项链的自然生成、装备、解咒、保存恢复及状态哈希闭环。
- 负向弹药保留正附魔但没有 Ego；负向装置可带 Ego 且不自动诅咒；Ordinary 箭袋保持基础容量。
  三类都由实际自然生成路径验证，不能统一套用装备的最终诅咒。
- 公共 RNG 短路、共享未知物品投影和哈希结构改变，统一刷新及验证 26 条 active exact
  fixture；保留场景命令和前置条件，政策零 waiver。

验证包含核心、内容、协议、存档、导入、本地化测试，Clippy、生成文件检查、内容锁及前端测试
与 UI 构建。未运行桌面 E2E，也未生成新的可玩桌面包，不计作人工试玩。

本次通过记录：核心全量 924 项通过、1 项既有忽略，随后新增类型分支测试 1 项单独通过；
内容 133、contract 7、导入 174、本地化 39、协议 7、存档 2、前端 181 项通过。
`cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings`、bindings/schema
`--check`、内容锁、`verify-all` 和 `validate-policy` 通过；26 条场景的命令与前置条件经比较无改动。
Ego 审计核对 160 个身份、139 个装备底材，`runtimeParityComplete` 继续为 false。

后续 E8.3–E8.8 继续按计划推进，本批不代表完整 E8 完成。
