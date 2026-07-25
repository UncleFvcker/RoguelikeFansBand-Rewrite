# Contract v71：RFB 角色构筑、种族职业与技能集合

状态：协议 1.71 / contract-v71 历史基准；内容包 1.63.0；state hash Schema v30；active baseline 已由 contract-v72 接替

## 范围

v71 在 v70 的等级、属性和 HP 成长闭环之上，建立原版 RFB 风格的角色来源层：Race、Class、Personality、技能定义、技能集合和可保存的初始构筑。它是角色创建与成长的基础，不试图一次复刻原版全部职业、法术和职业专属资源。

原版的 `skills_t`/`base_skills`/`extra_skills` 思路在这里拆成内容数据：一个 `SkillDefinition` 描述稳定技能种类，一个 `SkillSetDefinition` 描述一组 base/growth 修正，Race、Class、Personality 各引用一组技能集合；`CharacterBuildDefinition` 再把三者和出生属性、额外出生物品绑定成可选择的稳定构筑。

## 内容模型

内容包新增六个根目录和对应索引：

- `skills`：十个公共技能（解除、装置、豁免、潜行、搜索、感知、近战、远程、投掷、挖掘），每个技能有最大值、标签和本地化名称；
- `skillSets`：按技能 ID 声明 base 与每十级成长值；同一构筑中 Race、Class、Personality 的同名技能按稳定顺序相加；
- `races`、`classes`、`personalities`：属性修正、生命/经验倍率、基础 HP、技能集合和出生物品来源；
- `builds`：引用 Race/Class/Personality，声明六维出生自然属性和构筑专属出生物品。

所有 ID、引用、技能最大值、出生装备槽位和堆叠容量在内容编译阶段校验。编译器按 ID 排序根集合和技能条目，生成内容锁和 hash，不依赖文件系统遍历顺序。

demo 包为 1.63.0，包含 10 个技能、11 个技能集合、3 个 Race、5 个 Class、3 个 Personality 和 5 个构筑：

| 构筑 | Race | Class | Personality | 用途 |
| --- | --- | --- | --- | --- |
| Explorer | Human | Explorer | Ordinary | v70 兼容默认基线 |
| Vanguard | Human | Warrior | Combat | 近战和出生武器 |
| Scholar | Elf | Mage | Ordinary | 书本施法前置原型和高经验倍率 |
| Pathfinder | Elf | Ranger | Cautious | 远程/混合武器 |
| Tinkerer | Gnome | Artificer | Cautious | 装置使用者和出生护符 |

这些是可验证的代表性原型，不把原版几十个职业或完整法术表伪装成已经完成的内容规模。

## 派生规则

构筑创建时先保存 `buildId` 及其 Race/Class/Personality 身份，再按固定层次计算：

1. `CharacterBuildDefinition.attributes` 成为六维自然属性；
2. Race、Class、Personality 的属性修正按 Species → Class → Personality 顺序应用，并受当前 `3..18/xx` 阶段上限约束；
3. 三个来源的生命倍率、经验倍率按百分比相乘并四舍五入；基础 HP 修正、装备修正和 Constitution 生命倍率在 v70 派生管线中继续合并；
4. 三个技能集合的同名技能 base/growth 相加，当前值为 `base + growthPerTenLevels * level / 10`，再限制到技能 maximum；
5. Race → Class → Personality → Build 的出生物品按内容排序确定性分配 `generated.item.N`，不消耗模拟 RNG。装备槽位、物品知识和派生攻击/防御由现有物品管线处理。

目前已经消费的技能边界为：`disarming` 同时驱动开锁/解除，`search` 驱动搜索，`melee`、`ranged`、`throwing` 驱动相应攻击，`digging` 驱动挖掘。`device`、`saving-throw`、`stealth`、`perception` 已进入内容、存档和 UI，但等待后续规则纵切接入具体检定。

## 协议、存档与 hash

协议升级为 1.71：

- `PlayerDto.build` 暴露构筑、Race、Class、Personality 名称和合并倍率；
- `PlayerProgressDto.skills` 暴露当前/最大/base/growth 和本地化技能 ID；
- `PlayerSaveDto.build` 保存稳定身份，`PlayerProgressSaveDto.skills` 保存聚合结果；
- 缺少新字段的 v70 及更早存档按世界默认构筑迁移。技能列表按当前内容与保存等级重算；空技能列表表示旧存档字段缺失，而不表示一个真正的空构筑；
- 若保存的构筑身份、技能集合或等级派生结果不一致，载入拒绝，避免静默改变角色。

构筑身份、技能聚合、出生装备实例和知识状态进入 state hash Schema v30。HP 成长序列、正式 save 容器和 v70 的胜利后 100 级/`18/820` 解锁规则保持不变。出生装备的知识状态为 `tried=true, aware=true`，因此带装备构筑可以稳定存档回读。

## 确定性与兼容覆盖

内容 hash 为 `1c94890a0f39d42a4b496a7222b8c9d191f24fe94b3c9d47d4a1eeea5364c5b4`；v70 hash `ad6b35c6e0ae8980a74fac51ea1e6597b09559541d4a85d598284dc2cb41d7e6` 进入内置迁移白名单。迁移不重新生成楼层、不补出生物品、不推进正式 RNG；只恢复默认构筑身份并按等级重建技能。

历史 baseline 位于 [`tests/fixtures/contract-v71/scenarios`](../tests/fixtures/contract-v71/scenarios)，共有 152 个 exact fixtures、零 waiver。新增四个构筑 fixture 覆盖 Vanguard、Scholar、Pathfinder、Tinkerer 的出生身份、技能、属性、装备和 save round-trip；核心专项测试还覆盖技能十级成长、经验倍率、无 RNG 漂移、非法构筑 ID 和 v70 缺字段迁移。

## 明确不在 v71 的范围

- 完整原版 Race/Class/Personality 名单、职业选择界面和重建角色流程；
- 技能练习、技能下降、属性损伤/恢复、职业专属资源；
- 法力、能力书、法术失败率、书本阅读和完整法术系统；
- 怪物种族/职业、经济、商店和更大规模内容包。

可观察的 device、saving throw、stealth、perception 技能检定已由 [Contract v72](contract-v72-observable-skill-checks.md) 完成；后续转入法术/能力书基础。
