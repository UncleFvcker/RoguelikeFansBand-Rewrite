# 职业与种族导入交接

更新时间：2026-08-14
当前基线：`main@da8c43bc4`

本文是继续增加正式 RFB 职业与种族的当前操作入口。历史实现与逐批版本记录见
[`class-next-handoff.md`](class-next-handoff.md)，跨 worktree 的 ID 和版本协调见
[`parallel-worktree-handoff.md`](parallel-worktree-handoff.md)。二者与本文冲突时，先以当前代码和
`main` 为准，再更新本文，不从旧记录反推现状。

## 1. 当前基线

- demo pack：`1.344.0`
- content hash：`7930a9ba2980097431e039479334265842cb54bd143e279efde2c93fd47da96b`
- Protocol：`1.204`
- State Hash Schema：`v100`
- save 容器：`v1`
- active fixture baseline：`contract-v303`
- 正式内容：6 个 Class、9 个 Build、54 个 SkillSet、46 个 Race；其中 New Game 当前开放
  6 个职业构筑和 13 个种族。

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

内容包另有 `demo.build.high-mage-arcane`、`demo.build.high-mage-sorcery` 和
`demo.build.high-mage-armageddon`。它们保留领域扩展接口，但不在当前 New Game 构筑列表中；开放前必须
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

种族通过新游戏请求中的独立 `raceId` 覆盖 Build 的默认 Human。不要生成
“职业 × 种族”的重复 Build JSON。玩家外观目前由职业 Build 决定，新增普通种族不复制玩家 Actor 或
tileset 映射。

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
