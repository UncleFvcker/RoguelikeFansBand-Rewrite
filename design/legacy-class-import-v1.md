# 旧版职业与施法档案导入 v1

状态：已实现（P52，导入优先级规划 T3；纯工具迭代，协议、演示内容包与 state hash Schema 不变）。产物只写入 `.local/packs/rfb-legacy/{classes,skillSets,legacyMagicProfiles}/`，仓库不提交原版内容。

## 1. 固定来源与职业集合

导入器只读取固定 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 Git 对象，不读取原版工作树当前文件。`src/defines.h` 提供职业数字索引，`src/classes.c` 的唯一分派 switch 提供 53 个当前注册职业与 `*_get_class` 函数；`m_info` 另保留已退出当前注册表的 Imitator（索引 12），当前注册表另有尚无 `m_info` 行的 Ethereal Mimic（索引 53）。两者取并集后生成 54 个职业壳。

职业函数体继续复用 P48 的花括号配平和结构化赋值解析：

- 六维、life、base_hp、exp 进入 `ClassDefinition`；
- `skills_t bs/xs` 进入职业专属八技能 `SkillSetDefinition`；
- `caster_info` 在静态可读时提取施法属性、最低失败率、最低等级、三项负重参数与 `CASTER_*` 选项；依赖当前形态或技能分配的档案保留为 dynamic；
- birth、powers、专属 spells、等级钩子、装备限制等尚无通用消费者的函数指针进入 `classHookGaps`；
- Warlock、Weaponmaster、Skillmaster、Disciple 等依赖子职业或运行时分配的表面保留基础壳，并标记 `legacy-dynamic-shell`，不猜测某个子职业。

## 2. m_info 施法档案

53 份 `m_info` 档案完整保留：

- `I:book:stat:xtra:type:first:weight`；
- 12 个 `R:realm:readable` 领域可读性行；
- 每个可读领域的 32 条 `T:level:mana:fail:exp`，按原版声明顺序稳定编号 0–31。

标准化结果写入 `legacyMagicProfiles/<class>.json`；跨职业领域表写入 `realm-readability.json`；C 侧 `caster_info` 与 `m_info` 挂靠关系汇总到 `class-casting-shells.json`。这些文件是 P53+ 玩家领域法术映射的中间输入，刻意不列入 `pack.json.contentRoots`：当前 `ClassDefinition.castingProfile` 要求真实 resource、ability book 与 ability 引用，本轮不会用占位能力伪造可施放法术。

## 3. s_info 缺口

`s_info` 当前没有对应的逐武器熟练度、武术、双持与骑乘熟练度内容模型。本轮解析每个职业档案并将差异量化到 `classProficiencyGaps`，不把原版熟练度错误折算成八项通用技能。

## 4. 固定基线结果

- 职业：54/54 生成，53 个当前注册，53 个找到 C 源静态表面；
- m_info：53 份档案、636 个领域行、144 个可读领域行、4608 条逐法术参数；C 侧另提取 46 个 caster_info 壳，其中 5 个为动态档案；
- s_info：52 份档案、16640 条逐武器熟练度、156 条专项熟练度（武术/双持/骑乘各 52）；
- 本地包：54 classes、141 skill sets（67 种族 + 20 性格 + 54 职业），通过 `rfb-contentc inspect-source`；
- 合成 fixture 固定职业注册表连接、函数名后缀冲突、stats/skills/多行 flags、caster_info、m_info 可读/不可读领域、逐法术参数和 s_info 缺口计数。

## 5. P53–P55 承接状态

P53 选择 Death 第一册建立首个真实 ability、ability book、实体书、Mana 与职业 casting profile 纵切；P54/P55 已把第一、二册共 16 个槽位接入 12 个静态职业，逐职业等级/耗魔/失败率、职业效果缩放和 beam 几率通过 `abilityOverrides` 与 casting profile 保真，详见 [旧版玩家领域法术导入 v1](legacy-player-spell-import-v1.md)和 [Contract v105](contract-v105-death-second-book.md)。力量、敏捷、体质、生命施法以及职业专属 spells 继续保留在施法壳中，待对应通用 surface 覆盖后接入。
