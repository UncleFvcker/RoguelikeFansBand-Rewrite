# Ego 词条导入计划

更新时间：2026-08-14

工作分支：`codex/items-next`

本计划把 ego 作为独立的物品生成里程碑推进，并优先闭合 Craft 第四册「工艺」所依赖的
武器、护甲与弹药候选。领域内容在 ego 候选与实例化行为完整以前不开放「工艺」。

## 1. 当前基线

本次审计读取 `D:/codex/Frogcomposband` 的 Git `master` 对象；审计时解析到提交
`efd63661302866038f58d8cd2553b23e6af3bf9d`。后续实现仍以执行时最新的 `master` 为权威，
不能改读其工作树。

| 项目 | 当前结果 |
| --- | ---: |
| `e_info.txt` ego 总数 | 160 |
| 当前 importer 可生成 affix | 129 |
| 当前 importer 的 `ego-inexpressible` | 31 |
| 当前 importer 未实现的 ego activation | 13 |
| demo pack 正式 affix | 15 |
| Craft 类型兼容 ego | 122 |
| Craft 标准选择可达 ego（rarity > 0） | 121 |

Craft 类型兼容的 122 条记录按权威 `T:` 分类为：

| 批次 | 记录数 |
| --- | ---: |
| `WEAPON` / `DIGGER` | 30 |
| `AMMO` / `BOW` / `HARP` | 16 |
| 各护甲类型 | 76 |

其余 38 条属于 `RING`、`AMULET`、`LITE`、`QUIVER`、`DEVICE` 等非 Craft 类型。
这些记录仍属于完整 ego 方向，但不阻塞「工艺」。

当前 importer 的 129 条不是可直接整体接入正式掉落池的完成品：

- `W:` 第三项是 rarity；当前 parser 读取但未保存，运行时也没有使用；
- RFB 用等级偏差增加 rarity，仍保留至少 1 点权重；当前
  `affix_is_compatible_with_item` 把 `generationLevel/generationMaxLevel` 当作硬过滤；
- 当前 importer 把多数 `C:` 最大值直接写成固定顶格属性，未还原 `ego.c` 的实际随机实例化；
- 当前 loot table 只支持显式 `affixWeights`，`base-items` 只列出 Slaying 与 Protection，
  不能表达按物品类型从完整 e_info 集合选择；
- 当前 `craft-item` 从显式武器/护甲 ID 列表等概率选择，不能表达 RFB 的类型、等级和 rarity 权重；
- importer 尚未读取 `master:src/ego_name_zh.inc`，因此不能保证正式中文名与权威表逐项对齐。

## 2. 唯一权威来源

| Git 对象 | 用途 |
| --- | --- |
| `master:lib/edit/e_info.txt` | source index、英文名、适用类型、等级、最高等级、rarity、`C/F/E` 数据 |
| `master:src/ego_name_zh.inc` | 逐 source index 的权威中文显示名 |
| `master:src/init1.c` | `e_info` 字段解析语义 |
| `master:src/ego.c` | ego 选择权重、各类型选择顺序和实例化随机流程 |
| `master:src/object2.c` | `apply_magic` 的 quality、神器、ego 调度与 RNG 顺序 |
| `master:src/spells3.c` | Craft「工艺」调用的 `brand_weapon_aux` / `brand_armour_aux` |

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
- 现有 crafting scroll 的小型显式候选行为在迁移到共享选择器前保持不变，不增加第二套永久系统。

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

### E0：权威审计与契约基线

- importer 读取 `e_info.txt` 与 `ego_name_zh.inc`，建立 160 条逐 index 审计；
- 报告类型、rarity、动态分支、activation、未映射 flags 和中文 unresolved；
- 用真实 `master` 测试锁定 160 / 122 / 121 / 38 这些结构计数；
- 不新增正式 affix，不改变游戏行为。

提交目标：`test: audit authoritative ego catalog`

### E1：RFB ego 身份与选择核

- 增加可选 `sourceIndex/rarity/types` 元数据及严格验证；
- importer 正确保存 `W:` rarity 和 `T:` 类型；
- 实现 source-order、等级惩罚与 rarity 0 排除；
- 增加低于等级、区间内、高于最高等级、重名和多类型的确定性 RNG 测试；
- 尚不改 `base-items` 自然掉落。

提交目标：`feat: add authoritative ego selection metadata`

### E2：共享实例化底座

- 把当前 loot、`craft-item` 和指定 affix 路径共用的 materialize 逻辑收敛为一个 owner；
- 优先落到现有 `rolledAffixes/enchantments/damageDiceOverride/curse/activation` 字段；
- 锁定取消、失败、堆叠拆分、鉴定与 RNG 顺序；
- 不为尚未实现的 ego 添加占位效果。

提交目标：`refactor: share ego materialization`

### E3：近战武器与挖掘工具 30 条

- 按 `ego.c` 的 weapon/digger 分支逐项实现；
- 闭合普通属性、slay/kill、元素及特殊 brand、额外攻击、武器骰、祝福、抗性、诅咒和 activation；
- 未有真实消费者的 `VORPAL`、mana/order/wild brand 等在本批内实现后才开放对应 ego；
- 选择、属性和关键随机分支采用代表性 exact tests，不为 30 条复制同构测试。

提交可继续按行为族拆小，例如“基础武器词条”“特殊品牌与骰面”“诅咒与激活”；每个子批仍独立提交。

### E4：弹药、发射器与竖琴 16 条

- 对齐 ammo、bow、harp 的独立选择和实例化分支；
- 闭合 extra shots/might、Returning、Exploding、Endurance、Holy Might 等行为；
- 对堆叠弹药验证一次 ego 选择、一次实例化结果和整堆共享状态；
- 复用 Archer 已有弹药制造测试，但不把其中的小候选池当完整 ego 生成。

### E5：护甲 76 条

先实现 28 条涉及 body armor、dragon armor、shield、robe 的记录，再实现剩余 48 条仅属于
crown/helmet/cloak/gloves/boots 的记录；3 条跨组记录由前一批拥有，后一批不得重复定义。

本阶段重点闭合 reflection、元素伤害光环、revenge aura、随机抗性/高抗、sustain、telepathy、
levitation、magic resistance、spell/device power、诅咒和 activation。`ego_name_zh.inc` 名称、类型组合
和 property knowledge 必须随每个子批一起完成。

### E6：Craft「工艺」解锁

- 此时 122 条 Craft 类型定义全部存在，121 条标准候选行为完整；
- `craft-item` 增加 RFB ego policy，选择等级固定为玩家等级；
- 实现无名物品限制、非弹药单件限制、弹药 59 上限、31–59 确认与数量失败率；
- 成功后完全鉴定、记录 crafting origin 和 virtue；失败不留下 affix 或部分属性；
- 与自然生成共享选择和实例化，且明确断言不会生成神器。

该批完成后才继续 Craft 第四册和领域收口。

### E7：非 Craft ego 38 条

- 按 ring/amulet、lite/quiver、device 三个子批完成；
- 继续复用同一选择核和实例化入口；
- 只为真实 source 分支增加消费者，不因“凑齐 160”保留 no-op affix。

### E8：自然掉落与完整验收

- 为 `base-items` 增加 RFB ego policy，替代当前只有 Slaying/Protection 的演示池；
- 保留现有 quality、神器和 luck 调用顺序，对齐 great item 才进入 ego 的边界；
- 审计所有正式基础装备至少有正确类型映射，特殊/rarity 0 ego 不会自然出现；
- importer 达到 160 条有定义、标准池只含 rarity > 0，未实现 ego flag/activation 缺口归零；
- 删除被完整正式定义取代的重复 demo affix，只在 ID/行为确实相同且无调用冲突时合并。

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
- 完整 160 与自然掉落接通的 E8 才做一次里程碑级全量验收。

## 7. 完成定义

ego 方向完成时必须同时满足：

- [ ] 160 条 source ego 都有稳定 index 身份、权威英文名和权威中文名或明确 unresolved；
- [ ] rarity、类型和等级惩罚选择与 `ego.c` 一致，rarity 0 不进入标准池；
- [ ] 每条可生成 ego 都有真实属性消费者，不存在只显示名称的 no-op affix；
- [ ] 动态结果在物品实例中物化并经 save/state hash 稳定往返，不在读档时重掷；
- [ ] 自然掉落、Craft 和显式强制 ego 共用一个选择/实例化 owner；
- [ ] Craft 的 121 条标准候选完整，且永不生成神器；
- [ ] importer 的 `ego-inexpressible`、`unmappedEgoFlags` 和 `ego-activation` 归零；
- [ ] demo pack、内容锁、必要 schema/bindings 和直接受影响 fixtures 已按真实变化收口；
- [ ] 每个实施批次均有独立提交，没有混入其他领域或无关重构。
