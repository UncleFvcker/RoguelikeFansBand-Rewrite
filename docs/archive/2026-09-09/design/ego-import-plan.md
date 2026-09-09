> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# Ego 词条导入计划

更新时间：2026-09-09

工作树：`D:/codex/RoguelikeFansBand-Rewrite-realms-items`；工作分支：`codex/realms-items`。
E5.0 起始代码基线：`da7ba67be`，起始 main：`62f959f3b`。E0–E4 的完成说明保留历史批次版本；
E5 已接入护甲物化与消费者，E6 已完成工艺事务，E7 已接入 38 条非 Craft ego，当前内容为 1.391.0。
E8 已推进旧近似清理与集成检查；[完整审计](../../../../design/ego-integration-audit.md)列出六项仍未闭合的共享生成契约，尚不满足完整原版等价完成条件。

本计划把 ego 作为独立的物品生成里程碑推进，并优先闭合 Craft 第四册「工艺」所依赖的
武器、护甲与弹药候选。领域内容在 ego 候选与实例化行为完整以前不开放「工艺」。

## 1. 当前基线

本次实跑 `audit-egos D:/codex/Frogcomposband/master`，通过 Git 对象读取 `master`；解析到提交
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。后续实现仍以执行时最新的 `master` 为权威，
不能改读其工作树。

| 项目 | 当前结果 |
| --- | ---: |
| `e_info.txt` ego 总数 | 160 |
| E5.0 审计时的 currentImporterExpressible / Inexpressible | 134 / 26；历史审计值，不是运行时完成数 |
| e_info 显式 E: activation 记录 | 13；不含 ego.c 分支随机激活，也不是未实现数 |
| demo pack 正式 affix | 168；其中 160 条为唯一原版 source 身份，122 条为 Craft 类型定义 |
| 武器/工具、远程/竖琴 source-index 物化及自然生成路径 | 30 + 16；见 E3/E4 的入口限制 |
| 权威中文名 | 160 |
| 中文名 unresolved | 0 |
| Craft 类型兼容 ego | 122 |
| Craft 标准选择可达 ego（rarity > 0） | 121 |

Craft 类型兼容的 122 条记录按权威 `T:` 分类为：

| 批次 | 记录数 |
| --- | ---: |
| `WEAPON` / `DIGGER` | 30 |
| `AMMO` / `BOW` / `HARP` | 16 |
| 各护甲类型 | 76 |

其余 38 条为首饰 17、光源 9、箭袋 4、装置 7，以及 `SPECIAL` 1 条。
这些记录仍属于完整 ego 方向，但不阻塞「工艺」。

当前已具备权威中文表、rarity/source-order 选择核、共享原子物化、武器、远程和护甲消费者。
`base-items` 与工艺卷轴共用选择和物化入口，覆盖全部 Craft 类型；前者使用生成等级，后者使用玩家等级。

剩余缺口：

- E7 各非 Craft 类型已接入，但负向质量/诅咒、随机神器、首饰价值重试、龙系基础生成、背包和未开放职业专属修正仍待闭合，见 E8 审计；
- Craft 领域的 32 个法术及四册内容尚未导入，需要按领域流程另行审计和实施；
- 本次 160 个权威中文名全部可解；新基础物品仍需单独核对 `kind_name_zh.inc`。

当前顺序：**E0–E7 已接入 → E8 集成审计与共享缺口收口**。
巫师法杖的基础身份、普通获取和实际减耗已在 E8 复核补齐；Craft 领域本身仍按领域流程另外导入。

## 2. 唯一权威来源

| Git 对象 | 用途 |
| --- | --- |
| `master:lib/edit/e_info.txt` | source index、英文名、适用类型、等级、最高等级、rarity、`C/F/E` 数据 |
| `master:src/ego_name_zh.inc` | 逐 source index 的权威中文显示名 |
| `master:src/init1.c` | `e_info` 字段解析语义 |
| `master:src/ego.c` | ego 选择权重、各类型选择顺序和实例化随机流程 |
| `master:src/object2.c` | `apply_magic` 的 quality、神器、ego 调度与 RNG 顺序 |
| `master:src/spells3.c` | Craft「工艺」调用的 `brand_weapon_aux` / `brand_armour_aux` |
| `master:src/spells_c.c` | `crafting_spell` 的目标限制、数量确认、失败率、美德与来源 |

中文名只采用 `ego_name_zh.inc` 对应 source index 的字符串。该项为 `NULL` 时记录 unresolved，
不得按英文自行翻译。英文重名 ego 继续用 source index 消歧，不能依赖名称排序恢复原始顺序。

## 3. 必须保持的权威规则

### 3.1 选择

标准候选必须按 `e_info` source index 顺序枚举。对匹配类型的记录，RFB 权重为：

```text
adjusted_rarity = rarity
if level > max_level:
    adjusted_rarity += 3 * rarity * (level - max_level)
else if level < min_level:
    adjusted_rarity += rarity * (min_level - level)

weight = rarity == 0 ? 0 : max(10000 / adjusted_rarity, 1)
```

因此 `min/max level` 不是硬过滤。rarity 为 0 的 ego 不进入标准随机池，但仍应保留定义，供原版
显式强制 ego 或特殊生成路径使用。

自然掉落使用物品生成流程已经算出的生成等级；Craft「工艺」使用玩家等级。两者必须调用同一个
加权选择函数，只改变输入等级和允许类型，不能维护两套候选算法。

### 3.2 实例化

选择 ego 以后，必须继续执行 `ego.c` 对应类型分支。`C:`、pval、额外抗性、额外能力、命中/伤害/
护甲附魔、武器骰、诅咒、activation 等随机结果必须在生成时物化到物品实例；读档不得重掷。

优先复用现有实例字段：`affixIds`、`rolledAffixes`、`enchantments`、`damageDiceOverride`、`curse`、
`activation/charges` 和现有装备属性聚合。只有真实结果无法由这些字段表达时才增加新的持久字段。

不建设任意表达式或通用脚本 DSL。每一批只增加该批权威分支需要的最小 typed primitive，随后由
importer 从 source index/flags 生成对应内容。

### 3.3 可见性与生成入口

- ego 定义写入正式 pack 不等于立即进入自然掉落；一批只有在选择、实例化、中文名和消费者均闭合后
  才能接入生成策略；
- 固定奖励和法术指定烙印可以继续显式引用单个 affix，不受标准随机池影响；
- 自然生成与 Craft 均不能选择和基础物品类型不兼容的 ego；
- `AM_CRAFTING` 禁止固定神器和随机神器，不得把「工艺」变成神器生成入口；
- 「工艺」只能处理无 ego、无神器的合法武器/护甲/弹药，并保留原版堆叠弹药数量与失败规则；
- 现有 crafting scroll 已迁移到共享选择器，显式等概率候选路径已删除。

## 4. 数据与运行时设计

### 4.1 最小内容扩展

给正式 affix 增加可选的 RFB ego 生成元数据，至少包含：

- `sourceIndex`：稳定顺序与重名消歧；
- `rarity`：标准选择权重；
- `types`：权威 `T:` 类型集合。

继续复用现有 `generationLevel` 与 `generationMaxLevel` 保存 `W:` 的等级边界。普通原创 affix 不声明
这组元数据，绝不会因恰好带有 `weapon` 等 tag 而进入 RFB ego 池。通用 tag 仍用于 UI、过滤和旧的
显式 loot table，不作为权威 e_info 身份。

### 4.2 一个共享选择器

在现有物品生成 owner 中增加一个共享的 RFB ego 选择函数：输入物品定义、生成等级和允许的 ego
类型，输出一个 affix ID。函数按 `sourceIndex` 排序、计算原版权重并只消耗一次加权抽取 RNG。

自然掉落通过一个窄的 loot affix policy 调用它；Craft 事务调用同一函数。不要把 121 或 160 个 ID
复制进 ability/effect JSON，也不要突破当前 `affixWeights` 的 64 项限制来硬塞一张大表。

### 4.3 一个共享实例化入口

选择结果进入一个共享的 ego 实例化入口，负责：

1. 应用静态 affix 属性；
2. 按 source ego 的真实分支执行动态掷骰；
3. 写入 rolled properties、附魔、骰面、诅咒和 activation；
4. 设置 quality、鉴定知识和 origin；
5. 返回完整结果或原子失败，不留下半成品。

自然生成、Craft、crafting scroll 迁移和未来的强制 ego 奖励都复用此入口；调用方只决定选择策略、
目标与来源标记。

## 5. 实施批次与提交边界

每个批次完成后单独提交；未通过本批聚焦测试时不接入玩家可达入口。

### E0：权威审计与契约基线（已完成）

- importer 读取 `e_info.txt` 与 `ego_name_zh.inc`，建立 160 条逐 index 审计；
- 报告类型、rarity、动态分支、activation、未映射 flags 和中文 unresolved；
- 用真实 `master` 测试锁定 160 / 122 / 121 / 38 这些结构计数；
- 不新增正式 affix，不改变游戏行为。

提交目标：`test: audit authoritative ego catalog`

E0 已增加只读命令：

```powershell
cargo run -q -p rfb-legacy-import -- audit-egos D:/codex/Frogcomposband
```

命令通过 Git 对象读取 `master`，输出逐 source index 的英文名、中文名、类型、等级、最高等级、
rarity、标准可选性、Craft 类型、当前 importer 可表达性、物化输入、flags、未映射 flags 和 activation。
对审计提交 `efd63661302866038f58d8cd2553b23e6af3bf9d` 的实跑结果为 160 条全部有权威中文名、
122 条 Craft 类型、121 条 Craft 标准可选、38 条非 Craft、129 条当前 importer 可表达、31 条不可
表达和 13 条 activation。该批没有新增正式 affix，也没有改变游戏行为。

### E1：RFB ego 身份与选择核（已完成）

- 增加可选 `sourceIndex/rarity/types` 元数据及严格验证；
- importer 正确保存 `W:` rarity 和 `T:` 类型；
- 实现 source-order、等级惩罚与 rarity 0 排除；
- 增加低于等级、区间内、高于最高等级、重名和多类型的确定性 RNG 测试；
- 尚不改 `base-items` 自然掉落。

提交目标：`feat: add authoritative ego selection metadata`

E1 已给 affix 增加可选 `rfbEgo` 元数据；只有声明该元数据的定义才进入权威池。校验要求
`sourceIndex` 非零且全局唯一、`types` 非空且不重复，同时允许 `rarity` 为 0，供未来显式强制路径使用。
importer 已逐条写出 source index、rarity 和完整 `T:` 类型集合。

共享选择核由调用方提供精确允许类型与生成等级，按 source index 排序后应用 3.1 的等级惩罚公式；
标准池排除 rarity 0 和无元数据的原创 affix，有候选时只执行一次加权抽取。低于等级、区间内、超过
最高等级、英文重名、多类型、rarity 0 与原创 affix 均有确定性测试。该批未接入 `base-items`、Craft
或其他玩家可达入口，现有 pack 因可选字段省略而保持原 content hash。

### E2：共享实例化底座（已完成）

- 把当前 loot、`craft-item` 和指定 affix 路径共用的 materialize 逻辑收敛为一个 owner；
- 优先落到现有 `rolledAffixes/enchantments/damageDiceOverride/curse/activation` 字段；
- 锁定取消、失败、堆叠拆分、鉴定与 RNG 顺序；
- 不为尚未实现的 ego 添加占位效果。

提交目标：`refactor: share ego materialization`

E2 已把内容驱动 affix 的静态 ID、动态 `rollGroups`、activation 与 charges 收敛到同一个纯物化入口，
并由自然掉落、固定神器、任务奖励、世界显式物品和旧 `craft-item` 共同调用。各路径原有的 roll depth
策略和“先动态属性、后 activation”RNG 顺序保持不变；物化结果完整生成后才写入已有物品。

旧 `craft-item` 继续使用其小型显式候选池，但已通过共享入口提交结果，成功物品标记为 PlayerMade、
完全鉴定并保留堆叠拆分语义。拆分先分配实例 ID 再减少原堆数量；取消、非法目标或 ID 耗尽均不改变
物品或 RNG。该批未增加持久字段，也未给尚未导入的 ego 添加占位行为；法术烙印和造箭的专用动态
分支将在对应 ego 行为批次迁移，避免当前重复执行 `Slaying` 等 roll group。

### E3：近战武器与挖掘工具 30 条（已完成，保留基础物品入口缺口）

逐 index 审查与提交级实施方案见
[`design/weapon-digger-ego-import-plan.md`](weapon-digger-ego-import-plan.md)。审查确认 27 条含 `WEAPON`、
6 条含 `DIGGER`（其中 3 条跨类型、3 条仅 `DIGGER`），共 30 条且 rarity 全部大于 0。
source 1–27、40–42 的选择、拒绝重试、物化和相关消费者已实现。

本批已闭合普通属性、精确 Slaying/Craft、共享 pval、独立附魔、基础物品拒绝重试、近战骰面、
Mana/Vorpal/Order/Wild/Impact/Stun/Blessed、装备副作用、具体重诅咒和 activation。4 条显式 `E:`
之外还有 9 条分支随机 activation，去重后共 12 条可能带 activation；已有专用实例化和激活测试。
`WEAPON/DIGGER` policy 已开放。Mattock 已补入；Wizardstaff 的剩余入口缺口已在 E8 补齐，Mauler-only 重量调整仍待职业接入。

### E4：弹药、发射器与竖琴 16 条（已完成）

逐 index 审查与提交级实施方案见
[`design/ammo-launcher-harp-ego-import-plan.md`](ammo-launcher-harp-ego-import-plan.md)。审查确认 8 条
`BOW`、6 条 `AMMO`、2 条 `HARP`，16 条 rarity 均大于 0 且没有显式 activation。18 个基础物品身份、
Harp 生成期 pval、发射器倍率/射程/额外射击、完整六候选弹药池，以及 Returning、Exploding、Endurance
消费者均已接入。Archer 已复用共享选择器；`BOW/AMMO/HARP` 自然生成已开放。Bard 专属竖琴 pval
仍待职业实际接入时补齐，不宣称该职业已验收。

### E5：护甲 76 条

先完成 body armor/dragon armor/shield/robe 的 28 条，再完成剩余 48 条。
source 50/51/52 是跨组共享定义，由第一组负责；第二组只补对应类型的行为，不复制定义。

| 子批 | 范围与交付 | 开放条件 |
| --- | --- | --- |
| E5.0 已完成 | [76 条逐 index 审计](../../../../design/armor-ego-import-audit.md)；38 件现有护甲身份回填；静态契约、限制/动态/激活/消费者和稳定 ID | 357 件物品、65 个 affix；护甲 RFB Ego 池未开放 |
| E5.1 | 按已确认缺口扩展共享物化/装备聚合：affix 反射、必要光环、重量和基础 AC 变化；复用抗性、sustain、ESP、诅咒与 activation helper | 真实新增持久字段时独立交付公共底座，先合入 main，再继续依赖批次 |
| E5.2 | 盾牌 9 条：50–53、60–64；完整类型限制、随机抗性、反射、pval、附魔和 activation | SHIELD 池全部闭合后单独开放，不把 50–52 提前用于未完成的其他类型 |
| E5.3 | 身体护甲新增 70–77，加复用 50–53；长袍 80–82；可拆多个实现提交 | BODY/ROBE 调度一起验收后开放，保留 robe 的等级 30、1/7 特殊选择入口 |
| E5.4 | 龙鳞甲 85–92：子类型限制、基础吐息继承/替换、伤害和冷却修改、属性及光环 | DRAGON_ARMOR 完整池和所需基础龙甲接通后开放 |
| E5.5 | 剩余 48 条分头部、披风、手套、靴子批次；每批先补真实消费者再物化内容 | 每次开放一个完整类型池；跨类型的同一 source index 使用唯一 ID |
| E5.6 | 汇总 76 条、跨类型排除、普通获取、鉴定/属性知识、固定引用及存档；移除已替代的旧近似 | 122 条 Craft 类型全部有行为，121 条非零 rarity 的合法候选完整 |

第一组的准确身份集合为 `50–53, 60–64, 70–77, 80–82, 85–92`。
第二组为 `54–56, 95–104, 110–122, 125–130, 135–142, 145–152`；其中 source 103
rarity 为 0，保留真实强制用途，不能进入标准随机池。

E5.0 已交付 `audit-egos` 的 76 条静态契约检查和 `sync-demo-armor-ego-identities` 命令；
实跑读取 `master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，全部中文名可解。
聚焦测试覆盖源字段漂移、现有 ID 冲突、38 件基础身份与拒绝过早生成时的零 RNG；原有自然掉落测试和
11 个 equipment/inventory/tasks/town fixture 通过，无需刷新。当前不增加持久字段，不改 state-hash schema。

以下实际难点已记录在 E5.0 审计中：

- `ego.c:2851` 起的身体护甲分支会改变重量和基础 AC；不能只加一个固定 defense 顶值。
- `ego.c:2711` 的 Twilight 会改变基础 kind/sval 并清零基础 AC/to_a；不能只更换 affix 名称。
- `ego.c:2548` 的龙甲 Breath 继承基础 activation 后修改冷却和伤害，Lore/Death 可替换它；
  需要追到具体基础龙甲和激活消费者，不能把两次 activation 叠加。
- 当前 `player_reflects_bolts` 只读取基础物品/种族/状态；“基础物品已有反射”不代表 affix 反射已接通。
- `DEC_MANA`、`EASY_SPELL`、`SPELL_CAP` 分别追到费用、失败率和法力上限路径；复用后补 Wizardstaff
  的权威物品、中文名、获取、熟练度和 Arcane 生成。不能仅赋予显示词条。
- 第二组继续核对复仇/元素光环、双持、额外攻击、反魔法、自动鉴定和具体诅咒；已有能力只扩消费者缺口。

本页不预先规定一套“护甲状态框架”。每批按实际源码选择已有字段或最窄扩展，所有随机结果只在生成时物化。

### E6：Craft「工艺」事务（已完成）

- 此时 122 条 Craft 类型定义全部存在，121 条标准候选行为完整；
- `craft-item` 增加 RFB ego policy，选择等级固定为玩家等级；
- 实现无名物品限制、非弹药单件限制、弹药 59 上限、31–59 确认与数量失败率；
- 成功后完全鉴定、记录 crafting origin 和 virtue；失败不留下 affix 或部分属性；
- 与自然生成共享选择和实例化，且明确断言不会生成神器。

现有 crafting scroll 已走完整事务，替代了显式等概率候选路径。弹药使用 `randint1(30) > quantity - 30`
判定数量成功；确认提示使用原版整数百分比。确认绑定目标数量，过期确认按取消处理。
失败消耗已开始的使用，目标实例和属性保持原样；成功复用完整鉴定并设置工艺来源、折价和美德。
7 项核心专项、前端确认测试、内容和导入器检查覆盖此入口；版本与回归结果见[当前状态](current-status.md)。
Craft 领域当前尚未导入；本批只完成「工艺」所需的共享行为，不等于第四册或新游戏入口已开放。
后续领域工作仍先审计完整 32 法术，再按四册实施，不能直接挂一个孤立的第四册法术。

### E7：非 Craft ego 38 条（已接入）

身份、各类型生成与属性消费者、源版标记分类及 E8 边界见[非 Craft Ego 审计](../../../../design/noncraft-ego-import-audit.md)。
首饰 17、光源 9、箭袋 4、装置 7、特殊 1 均已接入；复用五个旧 ID。210、211、260 不进入普通随机池。
新增实例状态使用 Protocol 1.234、State Hash Schema v111、save header/payload v6，内容包 1.390.0。

| 子批 | 去重条数 | 范围 |
| --- | ---: | --- |
| E7.1 首饰 | 17 | 200–201、205–211、220–227；已有 sacred-pendant、magi-amulet、wizardry-ring 等先审计并复用 ID |
| E7.2 光源 | 9 | 235–243；光照/黑暗、燃料或持续时间、感知和激活的实际消费者 |
| E7.3 箭袋 | 4 | 265–268；容量、保护、无尽/相位箭袋行为，保持发射与弹药存量时点 |
| E7.4 装置 | 7 | 250–256；容量、恢复、难度、威力、保护和速度，按设备生成路径接入 |
| E7.5 特殊 | 1 | 260 `(Blasted)`；`spells3.c:4127` 的强制身份及实际状态变化，禁止加入普通随机池 |

总计 38。按实际类型调度复用选择核与物化，不把首饰/装置强行套入普通武器的 great-item 门槛。
已有定义不重复创建；rarity 0、强制生成和职业专属路径分别记录。E7 不阻塞 E6，但依赖的公共能力应复用 E5 底座。

### E8：自然掉落与完整验收（进行中）

本次结果见[集成审计](../../../../design/ego-integration-audit.md)和[160 条机器可读矩阵](../../../../design/ego-contract-audit.json)。
通用池及 12 个主题池已统一，旧配方与重复 Combat 已删除；160 条身份/实例检查和代表性桌面流程已落实。
六项共享契约尚未闭合，当前不能标记 Ego 方向全部完成。
具体按 [E8.1–E8.8 共享生成实施计划](../../../../design/ego-shared-generation-plan.md)推进，先补真实估值，再闭合依赖它的诅咒、随机神器与首饰重试。

- 收口各批已经开放的自然生成 policy，删除残留的旧近似随机池，不等到最后才一次性开放全部类型；
- 按 `obj_create_weapon/armor`、首饰、光源、箭袋和装置各自的源码调度核对 quality、神器和 luck/RNG 顺序；
- 审计所有正式基础装备至少有正确类型映射，特殊/rarity 0 ego 不会自然出现；
- 160 条 source 身份均有明确实现或原版特殊用途；标准池只含对应类型合法且 rarity > 0 的候选；
- 对每个剩余审计 flag/activation 记录“真实消费者、声明但无消费者、或明确未实现”；不得靠生成非空 JSON
  把完成数刷到 160，也不为 AWARE 这类源端无消费者标记编造功能；
- 删除被完整正式定义取代的重复 demo affix，只在 ID/行为确实相同且无调用冲突时合并。

E8 是明确的集成里程碑：运行 workspace/内容/生成物检查、完整契约回放，并安排代表性的桌面
“获得→鉴定→装备→触发效果/激活→保存恢复→工艺”流程。人工试玩由用户执行，单独记录。
Mauler/Bard 等尚未接入职业的专属行为仍需列明依赖，不能以当前职业测试声称所有原版职业均已覆盖。

## 6. 每批验证矩阵

日常只跑新增与直接相关检查：

```powershell
cargo test -p rfb-legacy-import ego
cargo test -p rfb-content affix
cargo test -p rfb-core ego
cargo test -p rfb-core item_generation
cargo run -q -p rfb-content --bin rfb-contentc -- inspect-source packs/rfb-demo-original
cargo run -q -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo fmt --check
git diff --check
```

只在真实变化需要时追加：

- 内容 schema 变化：生成并检查 content schemas；
- Protocol DTO 变化：推进 Protocol 并刷新 bindings/schema；
- 新持久字段：推进 save schema 与 State Hash Schema，运行聚焦 save/replay；
- 纯内容数量变化：只提升 pack 版本、刷新 content lock 和 README 数量；
- 共同初始化、RNG 主流程或共享投影变化：按基线策略刷新受影响 fixture 类别；
- 日常不做全量验收。状态哈希输入结构、共同初始化/RNG 或共享投影变化时，按根目录 AGENTS.md
  和 baseline-update-policy 执行所需完整验证；E8 另做一次明确的集成里程碑验收。

## 7. 完成定义

ego 方向完成时必须同时满足：

- [x] 160 条 source ego 都有稳定 index 身份、权威英文名和权威中文名或明确 unresolved；
- [x] rarity、类型和等级惩罚选择核与 `ego.c` 一致，rarity 0 不进入标准池；外围生成概率仍见 E8 缺口；
- [ ] 每条可生成 ego 都有真实属性消费者，不存在只显示名称的 no-op affix；
- [x] 已实现的动态结果在物品实例中物化并经 save/state hash 稳定往返，不在读档时重掷；
- [x] 自然掉落、Craft 和显式强制 ego 共用选择/实例化 owner；
- [x] Craft 的 121 条标准候选完整，且永不生成神器；
- [ ] importer 的每项未映射记录都有可核查分类，真实行为缺口归零；原版无消费者标记与 SPECIAL 不靠 no-op 消数；
- [x] demo pack、内容锁、必要 schema/bindings 和直接受影响 fixtures 已按真实变化收口；
- [x] 已实施批次均独立提交；尚未完成的 E8 共享契约明确保留，不混入其他领域。
