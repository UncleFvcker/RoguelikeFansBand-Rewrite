# 职业与种族导入交接

更新时间：2026-08-15
当前实现基线：`c8ca8e4d0`（屁精正式 New Game 开放；本次文档提交只做交接封板）

本文是继续增加正式 RFB 职业与种族的当前操作入口。历史实现与逐批版本记录见
[`class-next-handoff.md`](class-next-handoff.md)，跨 worktree 的 ID 和版本协调见
[`parallel-worktree-handoff.md`](parallel-worktree-handoff.md)。二者与本文冲突时，先以当前代码和
`main` 为准，再更新本文，不从旧记录反推现状。

## 1. 当前基线

- demo pack：`1.371.0`
- content hash：`c1eb19e4f44cc33dea8ba18ffa6ea266162723266cdc8bb27c2aa5517b791a36`
- Protocol：`1.222`
- State Hash Schema：`v104`
- save header/payload schema：`v2`（二进制容器格式仍为 v1）
- active fixture baseline：`contract-v303`，26 个 exact fixture
- 正式内容：6 个 Class、13 个 Build、65 个 SkillSet、57 个 Race；其中 New Game 当前开放
  6 个职业构筑和 35 个种族。

开始新批次前必须重新读取以上版本；本文中的数值是交接快照，不是永久常量。

### 正式职业

| 职业 | Class ID | New Game Build ID | 说明 |
| --- | --- | --- | --- |
| 战士 | `demo.class.warrior` | `demo.build.warrior` | 非施法基线 |
| 高阶法师 | `demo.class.high-mage` | `demo.build.high-mage-death` | New Game 当前只开放死亡领域 |
| 弓箭手 | `demo.class.archer` | `demo.build.archer` | 制造弹药与射击派生已闭合 |
| 圣骑士 | `demo.class.paladin` | `demo.build.paladin-death` | 死亡领域、随机祈祷学习 |
| 骑兵 | `demo.class.cavalry` | `demo.build.cavalry` | 骑术、坐骑成长和捕获球已闭合 |
| 狙击手 | `demo.class.sniper` | `demo.build.sniper` | 专注、特殊射击和探测怪物已闭合 |

内容包另有 `demo.build.high-mage-arcane`、`demo.build.high-mage-sorcery`、
`demo.build.high-mage-armageddon`、`demo.build.high-mage-nature` 和
`demo.build.high-mage-life`。它们保留领域扩展接口，但不在当前 New Game 构筑列表中；开放前必须
重新验收对应领域的书本、学习、施放、出生内容和 UI。

### 正式可选种族

New Game 当前按以下稳定 ID 开放：

- `demo.race.rfb-human`
- `rfb-legacy.race.half-orc`
- `rfb-legacy.race.high-elf`
- `rfb-legacy.race.dunadan`
- `rfb-legacy.race.barbarian`
- `rfb-legacy.race.hobbit`
- `rfb-legacy.race.kobold`
- `rfb-legacy.race.dwarf`
- `rfb-legacy.race.nibelung`
- `rfb-legacy.race.gnome`
- `rfb-legacy.race.half-giant`
- `rfb-legacy.race.half-troll`
- `rfb-legacy.race.half-titan`
- `rfb-legacy.race.cyclops`
- `rfb-legacy.race.yeek`
- `rfb-legacy.race.klackon`
- `rfb-legacy.race.dark-elf`
- `rfb-legacy.race.mindflayer`
- `rfb-legacy.race.imp`
- 龙人分支：
  - `rfb-legacy.race.draconian-red`
  - `rfb-legacy.race.draconian-white`
  - `rfb-legacy.race.draconian-blue`
  - `rfb-legacy.race.draconian-black`
  - `rfb-legacy.race.draconian-green`
  - `rfb-legacy.race.draconian-bronze`
  - `rfb-legacy.race.draconian-crystal`
  - `rfb-legacy.race.draconian-gold`
  - `rfb-legacy.race.draconian-shadow`
- `rfb-legacy.race.golem`
- `rfb-legacy.race.zombie`
- `rfb-legacy.race.skeleton`
- `rfb-legacy.race.wood-elf`
- `rfb-legacy.race.archon`
- `rfb-legacy.race.sprite`
- `rfb-legacy.race.snotling`

种族通过新游戏请求中的独立 `raceId` 覆盖 Build 的默认 Human。不要生成
“职业 × 种族”的重复 Build JSON。玩家外观目前由职业 Build 决定，新增普通种族不复制玩家 Actor 或
tileset 映射。

龙人专项已完成六个实现阶段，并在阶段 7 完成交接封板：修正旧 pack 断言；加入亚种变异覆盖、漂浮、
种族 AC、等级反射和职业候选过滤模型；加入红、白、蓝、黑、绿、青铜、水晶、金、阴影九个 Race、
SkillSet 与动态喷吐；
闭合八项普通 35 级力量；闭合“变形”的龙形身体、装备迁移、AC、爪击/撕咬、职业倍率、物理
变形免疫及 save/state-hash/replay 聚焦生命周期；最后增加正式选择标签和 New Game“龙人分支”。
九个 Race 现在在 35 级提供九选一并永久锁定：
龙皮、魔法抗性、龙之打击、致命吐息、再生、召唤同族、远古知识、抗性增加和变形。选择变形后，
身体按原版改为六个戒指槽及护符、光源、斗篷、头盔槽，失效装备通过既有迁移事务回包；龙形战斗与
AC 从出生龙人亚种、职业、等级和当前属性派生，不保存第二份形态状态。
九个 Race 均带 `rfb-compatibility`，通过既有 `raceId` 请求正式创建；仍刻意不带
`polymorph-candidate`，不会混入普通临时变形候选。玩家 Actor 与 tileset 继续由职业 Build 决定。
魔像专项也已封板：`rfb-legacy.race.golem` 现带正式选择标签并进入同一 `raceId` 请求与 Web 原生
种族选择器。它继续使用标准玩家 Actor/tileset，不增加职业过滤或“职业 × 种族”Build。
僵尸 `rfb-legacy.race.zombie` 已在后续 main 基线上正式开放：复用魔像的缓慢消化、1/20 食物营养、
装置吸收与满充能空手法杖路径，新增通用 `night-start` 出生标签和独立“恢复生命”能力。骷髅
`rfb-legacy.race.skeleton` 也已正式开放，复用同一亡灵底座，并在既有物品使用事务中闭合普通食物
漏到脚下、特殊食物消失及药水泼洒规则。
木精灵 `rfb-legacy.race.wood-elf` 已正式开放：静态矩阵和标准出生沿用既有内容模型，20 级
“自然感知”复用 `demo.ability-program.nature-awareness`，`forest-adapted` 标签通过统一当前有效
种族判定允许未骑乘木精灵以普通行动成本穿越树木；临时木精灵形态同步获得并在解除后失去能力与通行。
神使 `rfb-legacy.race.archon` 也已正式开放：飞行和看破隐形复用既有当前有效种族被动路径，没有
主动能力。原作 `p_ptr->align += 200` 仍没有统一玩家阵营模型；本批没有发明内容标签或局部状态替代，
导入审计继续把该语句保留为 `calc_bonuses` gap，等待真正的通用阵营系统。
小妖精 `rfb-legacy.race.sprite` 已正式开放：飞行、光抗和每 10 级速度 +1 复用既有当前有效种族
派生；12 级智力能力“睡眠粉”在 24 级及以下复用邻近睡眠，25 级起复用视野内状态结算。内容层只
新增内部动态步骤，等级结算后投影为既有协议效果，不增加新命令、Protocol、save 或 State Hash 字段。
屁精 `rfb-legacy.race.snotling` 已正式开放：1 级魅力能力“吞噬血肉”在前端确认后把饱食度设为上限
减一、增加 100 重度流血并承受最大 HP 三分之一伤害，且按原版可在混乱时使用。当前有效种族为屁精
时，任何蘑菇在普通效果之后共享一次 `L + randint1(L)` 持续时间，获得加速、石肤、英雄与巨力；
出生额外生成固定种类的 1–3 个“快速恢复蘑菇”，并保留标准口粮和火把。正式或临时屁精形态均被
蘑菇店拒绝购买。

### 龙人专项最终证据

- 最终实现提交：`ef212db26`（`Expose Draconian subraces in New Game`）。最终协调点为 pack
  `1.357.0` / content hash `43c8437b663e727646a077e75a1f7a55318651087062542ffa6e60fbe399108c`；
  Protocol `1.212`、State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。
- 阶段 6 只运行三项新增聚焦测试：核心九亚种正式创建、Web 九亚种分组、本地化 optgroup label；三项
  均通过。`verify-source`、相关 Rust 测试目标 `cargo check`、Web typecheck、Rust format 与 diff 检查
  通过。
- 阶段 3–5 的新增行为测试证据见 [`class-next-handoff.md`](class-next-handoff.md) 对应小节。依用户要求，
  本专项各阶段未运行或刷新全量测试与 fixture；后续主合并里程碑验收负责累计回归。

### 魔像专项最终证据

- 最终实现提交：`25b2bc0b6`（`Expose Golem in New Game`）。四个阶段提交依次为：
  `2e1c74061` 增加通用种族等级 AC/速度与 35 级生命力保护，`7e3114a7b` 增加构装体进食和吸收装置，
  `5e8fbee5c` 完成隐藏内容、石肤术与出生物品，`25b2bc0b6` 完成专项验收并正式开放。
- 最终协调点为 pack `1.361.0` / content hash
  `48db8bec826fc84f1b4b262ca621721b1ef729d4e78a79c4344914822d01d095`、Protocol `1.213`、
  State Hash Schema v104、save v2、active baseline `contract-v303`（26 个 exact fixture）。正式 New Game
  种族数从 28 增至 29；没有新增权威状态或 save 字段，也没有刷新 fixture。
- 权威静态矩阵、等级 AC/速度/生命力保护、毒抗/状态免疫/看破隐形与临时变形生命周期、缓慢消化、
  1/20 食物营养及附加效果、背包/脚下/空装置吸收与营养封顶、石肤术成功/失败/消耗/持续/AC、出生
  空手法杖和初始“正义”，以及吸收后的 save/state-hash/replay 均有聚焦测试通过。Web 聚焦测试覆盖
  魔像选项、`raceId` 请求和吸收装置入口。
- 本专项只运行魔像新增与直接相关测试、内容锁/schema 检查、相关 Rust 编译、Web typecheck 与协议
  binding 检查；按用户要求未运行全量 Rust/Web/fixture/replay 回归，累计验收留给主合并里程碑。

### 僵尸导入最终证据

- 实现提交：`2ecec68cd`（`Import Zombie race`）。最终协调点为 pack `1.366.0` / content hash
  `795d9e95d5285636b4a0273eb438f15d86f8ca6c4a7fb6ade99310ee094b9f2c`；Protocol `1.221`、
  State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。正式 New Game 种族数
  从 29 增至 30。
- 僵尸闭合六维、生命/HP/经验/红外/商店/技能矩阵，虚空与毒抗、5 级寒冷抗性、看破隐形、生命力
  保护、非生命/亡灵身份、缓慢消化、1/20 食物营养、装置吸收、夜间出生、无普通口粮、满充能空手
  法杖和初始“非生”。30 级感知能力“恢复生命”消耗 30、基础失败率 70%，恢复全部损失经验并
  增加 150 生命力；共享执行器现按原版 `lp_player(150)` 使用加法并封顶。
- 只运行了本批新增聚焦测试：内容 1 项、本地化 1 项、导入器 1 项、核心 5 项、Web 1 项，均通过；
  核心覆盖成功/失败支付、变形获得/失去、食物与装置代谢、夜间出生及 save/state-hash/replay。
  `verify-source`、Rust format 和 diff 检查通过。按用户要求未运行全量测试，也未刷新 fixture。

### 骷髅导入最终证据

- 实现提交：`be87dc1b2`（`Import Skeleton race`）。最终协调点为 pack `1.367.0` / content hash
  `a5d56b9c5e0f6c2fece100b5b117e363d0be4a78bab82f94927d5c48c3a8310d`；Protocol `1.221`、
  State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。正式 New Game 种族数
  从 30 增至 31。
- 骷髅闭合六维、生命/HP/经验/红外/商店/技能矩阵，碎片与毒抗、10 级寒冷抗性、看破隐形、生命力
  保护、非生命/亡灵身份、缓慢消化、装置吸收、夜间出生、无普通口粮、满充能空手法杖、30 级
  “恢复生命”和初始“非生”。
- 普通食物先执行治疗/恢复等魔法，再从骷髅下颚漏到结算后的脚下且不提供营养；蘑菇和精灵行粮在
  效果后消失，同样不提供营养。药水先执行饮用效果，再在当前玩家位置复用已有 `shatter_effect`
  路径；没有为药水另建推测性效果。临时骷髅形态按同一当前有效种族判定获得并失去这些规则。
- 只运行了本批新增聚焦测试：内容 1 项、本地化 1 项、导入器 1 项、核心 6 项、Web 1 项，均通过；
  核心覆盖食物分类与附加效果、药水破碎、等级被动、恢复生命、临时形态、夜间出生以及
  save/state-hash/replay。`verify-source`、Rust format 和 diff 检查通过。按用户要求未运行全量测试，
  也未刷新 fixture。

### 木精灵导入最终证据

- 实现提交：`9d3397869`（`Import Wood-Elf race`）。最终协调点为 pack `1.368.0` / content hash
  `cce98fbd13eb10f345494c1562b72170d511a0f6627371e93068d5405800efb5`；Protocol `1.221`、
  State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。正式 New Game 种族数
  从 31 增至 32。
- 木精灵闭合六维 `-1/+1/+2/+1/-1/+1`、生命 97%、基础 HP 16、经验 125%、3 格红外、商店 95%及
  八项技能矩阵，保持标准身体和标准出生。初始美德为“自然”；未骑乘时可无延迟穿过树木，临时形态
  复用同一当前有效种族判定。
- 新增并由种族方向拥有 `rfb.ability.race.wood-elf-nature-awareness`，复用既有
  `demo.ability-program.nature-awareness`。该智慧能力在 20 级开放，消耗 15、基础失败率 50%，继续
  精确执行地图、陷阱、门、上下楼梯和普通怪物侦测，没有增加第二套侦测执行器。
- 只运行了本批新增聚焦测试：内容 1 项、本地化 1 项、导入器 1 项、核心 3 项、Web 1 项，均通过；
  核心覆盖能力等级/消耗/完整侦测、树木通行、临时形态、美德及 save/state-hash/replay。
  `verify-source`、Rust format 和 diff 检查通过。按用户要求未运行全量测试，也未刷新 fixture。

### 神使导入最终证据

- 实现提交：`7dafd6c6f`（`Import Archon race`）。最终协调点为 pack `1.369.0` / content hash
  `ca4c7b26e1bf204efefadedd2f116f95f2d4d713aeec543c417947033a68542b`；Protocol `1.221`、
  State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。正式 New Game 种族数
  从 32 增至 33。
- 神使闭合六维 `+2/0/+4/+1/+2/+3`、生命 103%、基础 HP 22、经验 200%、3 格红外、商店 90%及
  八项技能矩阵，保持标准身体和标准出生，初始美德为“正义”。飞行、看破隐形和临时形态生命周期
  复用既有当前有效种族判定；没有主动能力，也没有新增内容 ID、协议、存档或 State Hash 字段。
- 原作玩家阵营 `+200` 明确保留为 importer `calc_bonuses` gap；当前工程没有统一玩家阵营模型，本批
  没有以 `good` 标签、独立 Archon 特判或不可持久化状态伪造该行为。
- 只运行了本批新增聚焦测试：内容 1 项、本地化 1 项、导入器 1 项、核心 2 项、Web 1 项，均通过；
  核心覆盖永久/临时形态的飞行、看破隐形、红外、美德及 save/state-hash 往返。`verify-source`、
  Rust format 和 diff 检查通过。按用户要求未运行全量测试，也未刷新 fixture。

### 小妖精导入最终证据

- 实现提交：`638bab21c`（`Import Sprite race`）。最终协调点为 pack `1.370.0` / content hash
  `6923cb3c4cf41abd17e2ab03046b38dc0a027027b08a58c1ea648ec83fd510d5`；Protocol `1.221`、
  State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。正式 New Game 种族数
  从 33 增至 34，ability 数从 1831 增至 1832。
- 小妖精闭合六维 `-4/+3/+3/+3/-2/-2`、生命 92%、基础 HP 14、经验 135%、4 格红外、商店 90%、
  八项技能、光抗、飞行、每 10 级速度 +1、标准身体/出生与初始“自然”。临时小妖精形态按同一当前
  有效种族路径获得并在解除后失去飞行、光抗、速度与种族能力。
- 新增并由种族方向拥有 `rfb.ability.race.sleeping-dust` 和
  `rfb.ability-program.race.sleeping-dust`。12 级智力能力“睡眠粉”消耗 12、基础失败率 50%；24 级及
  以下影响半径 1 内怪物，25 级起影响视野内怪物，睡眠强度使用当前等级。内部 `sleeping-dust` 步骤
  在等级结算时直接变为既有 `Sanctuary` 或 `VisibleApplyStatus`，没有新增协议效果类型或第二套睡眠事务。
- importer 映射飞行、光抗、等级速度和睡眠粉；粗粒度 `calc_bonuses` hook 仍保留审计记录，但没有
  未表达的 Sprite 回调语句。只运行本批新增聚焦测试：内容 1、本地化 1、importer 1、核心 3、Web 1，
  均通过，覆盖静态矩阵、9/10/19/20 级速度、永久/临时被动、11/12 与 24/25 级能力边界、资源支付、
  目标集合、美德及 save/state-hash 往返。`verify-source`、schema、Rust check/format 和 diff 检查通过；
  按用户要求未运行全量测试，也未刷新 fixture。

### 屁精导入最终证据

- 实现提交：`c8ca8e4d0`（`Import Snotling race`）。最终协调点为 pack `1.371.0` / content hash
  `c1eb19e4f44cc33dea8ba18ffa6ea266162723266cdc8bb27c2aa5517b791a36`；Protocol `1.222`、
  State Hash Schema v104、save v2 和 `contract-v303` fixture baseline 均未改变。正式 New Game 种族数
  从 34 增至 35，ability 数从 1832 增至 1833。
- 屁精闭合六维 `-2/-2/-2/-2/-2/-5`、生命 85%、基础 HP 10、经验 45%、2 格红外、商店 125%、
  八项技能、标准身体/出生及初始“荣誉”。出生不是随机蘑菇种类，而是按原版生成 1–3 个
  `demo.item.fast-recovery-mushroom`，同时保留标准口粮和火把。
- 新增并由种族方向拥有 `rfb.ability.race.devour-flesh` 和
  `rfb.ability-program.race.devour-flesh`。1 级魅力能力“吞噬血肉”消耗 0、基础失败率 0%，使用专用
  `DevourFlesh` 投影表达饱食、流血和基于最大 HP 的自伤；`usable-while-confused` 只放宽该能力的混乱
  限制。前端使用原版确认文案，取消时不提交命令，replay 仍只记录已确认的施放。
- 当前有效屁精形态使用任何带 `mushroom` 标签的物品时，在普通物品效果后用一次 RNG 为加速、石肤、
  英雄和巨力应用相同持续时间；临时形态获得、解除后失去该规则。蘑菇店同时拒绝正式出生种族和
  临时屁精形态的购买请求。
- 只运行了本批新增聚焦测试：内容 1、本地化 1、importer 1、核心 5、Web 2，均通过，覆盖静态矩阵、
  出生数量、能力投影/混乱/饱食/流血/自伤/确认、蘑菇四重增益与共享 RNG、临时形态、蘑菇店、美德及
  save/state-hash/replay。`verify-source`、schema、Protocol bindings、Web typecheck、Rust
  check/format 和 diff 检查通过；按用户要求未运行全量测试，也未刷新 fixture。

## 2. 权威来源与不可变规则

1. 新规则和内容以 `D:/codex/Frogcomposband` 的 Git ref `master` 为权威；只能通过 Git 对象读取，
   不能读取该仓库当前工作树。例如：

   ```powershell
   git -C D:/codex/Frogcomposband show master:src/races_a.c
   git -C D:/codex/Frogcomposband show master:lib/edit/s_info.txt
   ```

2. 中文显示名必须逐字采用 RFB `master` 的运行时中文表或源码字符串。原版没有中文名时标记为
   unresolved，不自行翻译。
3. 一次只导入一个正式职业。职业必须交付完整纵切，不能只增加静态 Class 数据后把标志性机制留空。
4. 新增物品、能力、Ability Program、材料、词缀、资源或 Actor 前，先在任务计划和
   `parallel-worktree-handoff.md` 声明具体 ID、语义与所有者。items 方向已经拥有的内容只引用同一 ID，
   不复制或改名导入。
5. 现有 ID 一旦进入正式内容、存档或 fixture 就保持稳定。不要用 RFB 的数字序号作为运行时身份；数字
   序号只保留在 importer 审计层。
6. 项目不兼容旧开发存档。除非用户明确要求，不添加迁移、回退或双读路径。
7. 不因旧 gap 文档仍列出某项就重复实现。`legacy-class-import-v1.md` 等文件包含历史快照，先搜索当前
   模型、核心和测试确认真实缺口。

## 3. 职业导入清单

### 3.1 原版审计与开工门槛

- 从职业源码、`s_info.txt`、施法领域表和出生逻辑记录：六维、生命倍率、基础 HP、经验倍率、八项技能、
  宠物维持、骑术、逐武器熟练度、施法参数、等级被动、主动能力、装备限制及出生内容。
- 把每个职业身份机制分为“现有系统可表达”“需要窄扩展”“依赖其他方向”。只要标志性机制仍不能忠实
  表达，就先实现共享底座或将整个职业标记为 blocked，不以说明文字代替行为。
- 确认出生所需 item ID 已存在。若缺失，先声明 ID 和 items 所有权，再决定等待合并或协调导入。
- 魔法职业必须按完整领域纵切验收。Build 保留领域组合接口，Class 不硬编码某一本书；领域内容仍由
  items/realm 方向协调。

### 3.2 内容纵切

一个正式职业通常至少包括：

- `packs/rfb-demo-original/classes/<class>.json`
- `packs/rfb-demo-original/builds/<build>.json`
- `packs/rfb-demo-original/skillSets/<skill-set>.json`
- 所需 `abilities/*.json` 与 `abilityPrograms/*.json`，优先复用语义完全相同的现有 Program
- 四层出生内容：Class、Build、Race、Personality 的合并结果及任务 `classOverrides`
- `locales/en-US/content.ftl` 与 `locales/zh-CN/content.ftl`
- `web/src/session-shell.ts` 的 `PLAYTEST_BUILD_IDS`
- 新职业玩家 Actor，以及当前三套 tileset 的映射

Class、Build、SkillSet 和能力参数必须由内容数据承载。可复用的职业规则应成为窄的通用字段或 resolver，
不要在核心散落 `class_id == ...` 分支。

### 3.3 行为验收

- 同一 Human 下，新职业的属性、HP、生命/经验倍率、技能、资源和出生物正确。
- 逐武器、骑术、施法或其他职业熟练度采用该职业自己的初始值和上限。
- 每项主动能力覆盖开放等级前后、属性、费用、失败率、成功/失败支付、行动能量、目标取消和 RNG 顺序。
- 每项被动覆盖获得等级、失去条件、装备/状态交互和 save/replay。
- 出生随机数量、装备槽、物品实例来源与身份可复现。
- New Game 能选择该 Build；职业说明、能力分组、角色面板、玩家 Actor 和 tileset 正确。
- 若新增领域 Build，验证书本分组、学习模式、遗忘/记忆、施放与出生第一册，不只验证 Ability 存在。

## 4. 种族导入清单

### 4.1 内容与身份

- 优先完善 `packs/rfb-demo-original/races/` 和 `skillSets/` 中已有的 `rfb-legacy` 定义，不创建第二个
  Race 或 SkillSet ID。
- 精确导入六维、生命倍率、基础 HP、经验倍率、商店倍率、八项技能、红外视觉、kin、身体类型、抗性、
  状态免疫、属性维持、再生及其他种族被动。
- 正式可选种族必须带 `rfb-compatibility`；当前普通人形种族同时使用 `humanoid` 和
  `standard-body`，并保留已有 `legacy-import`、`polymorph-candidate` 等有效标签。
- 初始美德、等级奖励和天生能力必须来自 RFB 原版。没有 30 级天赋的种族不得套用 Human 奖励池。

### 4.2 种族能力与变形边界

- 种族能力使用 `RaceDefinition.abilities`，能力来源投影为 `Race`。语义相同才复用 Ability Program；
  种族能力与书本法术即使效果相同，也通常保持不同 Ability ID。
- 当前有效种族决定红外、抗性、维持、再生、看破隐形和种族能力，因此临时变形会获得并在解除时失去
  这些效果。
- 等级变异奖励属于角色出生种族。临时变形不会获得目标种族的等级奖励；降级后已锁定奖励不移除，
  再升级不重复授予。
- 已获得的知识状态不应因能力来源消失而删除。例如怪物探测记录会在解除半泰坦变形后保留。

### 4.3 UI 与验收

- 将稳定 Race ID 加入 `web/src/session-shell.ts` 的 `PLAYTEST_RACE_IDS`，并补 New Game 中英文案。
- 不为普通新种族创建玩家 Actor 或 tileset 副本；只有身体/外观系统真的需要时才扩展。
- 聚焦测试至少覆盖：静态数值、种族被动、初始美德、能力等级边界、资源支付、临时变形获得/失去、
  等级奖励不随变形、存档/replay，以及 Web 正确提交 `raceId`。
- 核心必须拒绝通过请求注入未带正式选择标签的 legacy Race。

## 5. 共享实现边界

优先使用这些既有能力，不另建平行系统：

- Class/Build/SkillSet、四层出生物合并、职业能力与等级被动
- chosen / divine-random 学习模式和多领域 Build 接口
- 武器、挖矿、骑术熟练度及角色面板投影
- 统一近战、projectile、物品生成、地形变更和 Ability Program resolver
- 种族能力、种族等级变异奖励、属性维持、红外、看破隐形、抗性和状态免疫
- polymorph 的“当前有效种族”派生

只有出现当前正式内容确实需要、且现有字段无法表达的行为时，才增加最窄的通用字段。若需要新权威状态，
同一提交必须完成初始化、严格读档校验、save、state hash、replay 和清理生命周期；不要先放未持久化的半成品。

常见接入点：

- 内容模型：`crates/rfb-content/src/definitions/characters.rs`
- 内容校验：`crates/rfb-content/src/validation/characters.rs`
- 原版导入审计：`crates/rfb-legacy-import/src/content.rs`
- 初始化与成长：`crates/rfb-core/src/game/progression.rs`
- 属性与种族被动：`crates/rfb-core/src/game/player_stats.rs`
- 主动能力：`crates/rfb-core/src/game/player_abilities.rs`
- 存档：`crates/rfb-core/src/game/persistence.rs`
- 新游戏入口：`web/src/session-shell.ts`、`web/index.html`
- 内容本地化：`locales/en-US/content.ftl`、`locales/zh-CN/content.ftl`

## 6. 版本与契约判断

| 变更 | 必须处理 |
| --- | --- |
| 仅内容 JSON/本地化 | 推进 pack 版本，重建并验证 `content.lock.json` |
| 内容 schema/定义字段 | 更新模型、校验、生成的 schemas、importer 审计；再推进 pack |
| 新命令或共享 DTO 投影 | 推进 Protocol，重生成 Rust schema/TypeScript bindings，并做聚焦前端测试 |
| 新权威 state-hash 输入 | 推进 State Hash Schema，更新 save 严格校验并刷新受影响 fixture |
| 公共初始化或 RNG 顺序变化 | 评估并刷新所有受影响的 active fixture 类别 |

State Hash v62 起不再包含 `contentHash`，所以纯内容改动不能以“hash 变化”为由全量刷新 fixture。

## 7. 聚焦验证

遵循当前约定，日常批次只运行新增和直接相关测试；不运行全量 fixture、桌面 E2E 或全量 replay。合并验收、
公共初始化/RNG/协议/State Hash 变更，或用户明确要求时再扩大范围。

按实际改动选择命令，不机械全部执行：

```powershell
cargo fmt --all -- --check
cargo test -p rfb-content <新增内容测试名>
cargo test -p rfb-core <新增行为测试名>
cargo test -p rfb-legacy-import <新增导入测试名>
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
```

```powershell
cd web
node --test --test-name-pattern="<新增职业或种族>" src/session-shell.test.ts
npm run typecheck
npm run check:protocol
```

提交前至少执行 `git diff --check`，确认只包含本方向的修改，并记录哪些聚焦测试已运行、哪些全量检查明确留给
合并验收。

## 8. 每批交接回填模板

完成一个职业或种族后，在 `class-next-handoff.md` 追加以下信息，并同步跨方向内容到
`parallel-worktree-handoff.md`：

```text
批次：<正式中文名>
权威来源：<master 中读取的源码/数据文件>
正式 ID：<Class/Build/Race/SkillSet/Ability/Program>
新增 ID 与所有者：<逐项列出；没有则写“无”>
复用 ID：<尤其是 items/realm 方向内容>
闭合行为：<静态、能力、被动、出生、UI>
明确未做：<必须是真实依赖，不能是职业身份缺口>
版本：<pack/hash/protocol/state-hash/save/baseline>
验证：<实际运行的聚焦测试与结果；注明未运行全量 fixture>
提交：<commit hash>
```

交付时工作树应干净。若仍依赖另一方向，写明所需的确切 ID、接口、最小版本或提交，不用“等 items 完成”
这类无法验收的描述。
