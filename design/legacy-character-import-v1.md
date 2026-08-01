# 旧版角色内容导入 v1（b_info / 种族 / 性格）

状态：已实现（P48，导入优先级规划 T1；纯工具迭代——协议/契约/演示包零变更）；产物只进 `.local/packs/rfb-legacy/{races,personalities,skills,skillSets}/`。

## 1. 来源与提取方式

种族与性格在旧版是**代码侧**内容（races*.c、beorning.c 等独立文件、personality.c），无数据文件。导入器按钉死 commit 对 `src/*.c`（按文件名排序保证确定性）做**结构化提取**：

- 函数头识别：`race_t *X_get_race(...)` 定义行（含返回类型、无分号——排除原型与调用点）→ 花括号配平取函数体；性格同理（`personality_ptr _get_X_personality`，仅扫 personality.c）。id 取函数名 snake→kebab（中文 `me.name` 不参与 id，文案导出另行排期）。
- 赋值行解析：`me.stats[A_*] = N;`、`me.skills.dis/dev/sav/stl/srh/fos/thn/thb = N;`、`me.extra_skills.*`、`me.life/base_hp/exp/infra = N;`、`me.flags = A | B;`。**右值非整数字面量（如怪物种族的 `100 + 5*rank`）标记 dynamic**，整条按 `race-code-dynamic` 跳过入报告——静态内容表达不了 rank 成长。`+=` 调整行计 `dynamic-adjustment` 钩子缺口。
- 函数指针赋值（`me.calc_bonuses/birth/get_powers/gain_level/...`）→ `raceHookGaps` 按钩子名计数；`me.flags` 记入 `unmappedRaceFlags`（等 T2 旗标系统）；`infra` 无 RFB 字段计入钩子缺口；`shop_adjust` 忽略（商店系统外）。

## 2. 映射

- **种族** → `rfb-legacy.race.*`：六维→modifiers（非零项，钳 ±100）、life→lifePercent（钳 25-400）、exp→experiencePercent（钳 25-500）、base_hp→baseHp（钳 ±1000）、八项技能→专属 skillSet（dis→disarming/dev→device/sav→saving-throw/stl→stealth/srh→search/fos→perception/thn→melee/thb→ranged，base 直映、extra_skills→growthPerTenLevels）、**bodySlots = b_info Standard 映射**。
- **性格** → `rfb-legacy.personality.*`：同构（无 bodySlots）。
- **技能花名册**：8 条 `rfb-legacy.skill.*`（kind 与 RFB 枚举 1:1，maximum 1000），有角色产物时生成。
- **b_info** → 槽类型映射：WEAPON_SHIELD 交替 weapon/shield（第 1/3/5…只手→weapon）、BOW→launcher、RING→ring（同类型多实例编号 ring-1/ring-2…）、AMULET/LITE/BODY_ARMOR/CLOAK/HELMET/GLOVES/BOOTS→amulet/light/body/cloak/head/gloves/boots；ANY/QUIVER/CAPTURE_BALL 无 RFB 对应，计 `bodySlotGaps`。全部 113 模板做缺口普查；**当前唯一绑定面是玩家种族×Standard**（12 槽，**刻意不含 RFB 原创 charm 槽**），其余模板经 r_info `Body:` 挂在怪物上，等附身/怪物种族玩法。

> contract-v151 production amendment：上述 12 槽仍是 P48 批量导入工具的历史输出。用户指定的 `demo.race.rfb-human` 不经该批量产物，改为显式 13 槽 RFB Standard：在同一交替手槽近似上补回 `quiver` 实例；当前箭矢仍留在库存，直到箭袋行为实现。它仍不含原创 `charm`。

## 3. 实测（钉死 commit）

- 种族：88 个 `*_get_race` 定义中 **67 个规整块导入**（21 个 rank 动态怪物种族按 race-code-dynamic 跳过）；抽查霍比特六维/技能/生命/经验与原版逐项一致。
- 性格：21 个中 **20 个导入**（1 个动态跳过）。
- 身体：113 模板全解析；缺口普查 any 76 / quiver 13 / capture-ball 4。
- 钩子缺口头部：calc_bonuses 76、birth 27、get_powers 等——T2 旗标系统与种族能力线的覆盖数依据。
- 产物 67 races + 20 personalities + 8 skills + 87 skillSets 随全包过 `inspect-source` 校验（四个 root 动态入 pack.json）。

## 4. 遗留

- 种族/性格旗标与钩子（抗性/免疫/能力/出生逻辑）→ T2 装备/内在旗标系统 + 后续种族能力线；
- 怪物种族 21 个（rank 动态）→ 怪物种族玩法设计后专项；
- demigod/draconian 子种族分支未展开（基础块已导）；
- 中文名/描述导出为 Fluent 片段（与物品线同一方向）。
