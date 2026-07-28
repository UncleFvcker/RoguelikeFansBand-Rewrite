# Contract v117：怪物、亡灵、宠物与同族召唤卷轴

状态：已实现

Contract v117 接入原版卷轴 sval 4/5/6/54：Summon Monster、Summon Undead、Summon Pet 与 Summon Kin。协议为 `1.117`，demo 内容包为 `1.108.0`，state hash Schema 保持 `52`，active baseline 包含 420 条 exact fixtures、零 waiver。内置内容 hash 为 `0b9023398c8213f9e74d7f0d4d076b8ce70819dbb5cd8cc4eb3a2b84d4996210`。

## 1. 内容模型

Race 新增可选 `kinCategory`，用于把当前有效 Race 映射到一个 actor tag。物品效果新增 self-only 的 `summon-category`：

- `selector` 可为 `any-monster`、显式 `category` 或 `player-kin`；
- `maximumLevelSource` 可为 `dungeon-depth` 或 `player-level`；
- 普通数量、群体概率/数量、敌对、unique 许可、半径和生命周期均显式声明；
- 物品召唤首版只允许 `durationTurns: 0`，避免把物品 ID 伪装成 ability 来源。

demo 的 Human、Elf/Gnome 与 Vampire Lord 分别映射到原版 glyph 风格的 `kin-glyph-112`（`p`）、`kin-glyph-104`（`h`）和 `kin-glyph-86`（`V`）。对应怪物以相同 tag 进入同族候选池。

## 2. 原版四种卷轴语义

Summon Monster 与 Summon Undead 使用当前地牢深度作为最高怪物等级，深度至少按 1 处理；结果敌对，允许 unique，但永远排除 guardian。Summon Undead 额外要求 `undead` category。

Summon Pet 使用地牢深度、排除 unique，生成永久玩家控制实体。Summon Kin 使用玩家等级、读取当前有效 Race 的 `kinCategory` 并排除 unique，同样生成永久玩家控制实体。候选按稳定 actor kind ID 枚举；等级、类别、guardian、unique 和当前存活 unique 占用全部在抽样前过滤。

永久友方只保存 `controllerId`，不创建带 ability 来源和倒计时的 `SummonIdentity`。类别候选、unique 可用性、落位、群体骰和实体创建由能力与物品共用同一组 helper，避免两条召唤路径漂移。

## 3. 零结果与知识

没有候选或附近没有空间时，卷轴仍被消费并推进一次正常行动；结果事件返回空 `entityIds/positions/summonedKindIds`。这两条零结果路径不抽召唤 RNG，未鉴定卷轴只从 Unknown 进入 Tried。只有实际生成至少一个实体时才进入 Aware。

协议新增 `GameEventOutcomeDto::ItemSummon`，复用 `AbilitySummonResolutionDto` 返回 owner、类别、敌友、群体、稳定实体 ID、位置和实际 kind ID。静态使用与设备激活分别有成功/无结果事件键；Web 只格式化瞬时结果，不把显示状态写入存档。

## 4. Legacy importer 与明确差异

legacy importer 将 sval 4/5/6/54 映射到上述四种效果，为每只导入怪物增加 `kin-glyph-<Unicode codepoint>`，并为每个导入 Race 增加 `kinCategory`。固定旧版提交的真实导入结果为 937 items、128 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；`scroll-effect` 从 38 降至 34。严格源校验、二进制编译和产物回读 hash 均为 `fbe1a9682d464e28ade0bd5df8fe8fbdda4fd1030413dd78965a4a4c983834d0`。

首版 Race-to-glyph 表按原版常规种族身份给出稳定代表值；依赖运行时形态或源码分支决定 glyph 的动态怪物种族使用固定代表值，不复制旧全局状态。Race 的 category 即使当前包没有候选也可产生合法零结果；本次真实包的全部引用均通过严格校验。

## 5. Fixtures 与验证

`contract-v117` 从 v116 迁移 413 条历史场景，并新增 414–420：

- 浅层通用敌对召唤与怪物等级边界；
- 浅层亡灵和低等级 Human Kin 的零候选、零召唤 RNG 与 Tried 知识；
- Pet、四级 Human Kin、一级 Gnome Kin 的类别选择、永久控制和存档回读；
- 四周封闭时的零空间消费、零召唤 RNG 与零 Awareness。

核心单测另固定亡灵 8/32 级池、Human/Gnome 类别、玩家等级扩池、guardian 排除、敌对 unique 许可/占用和永久友方回档。验证至少包括 workspace test/check/clippy、Schema 与 TypeScript bindings、demo/真实包源编译和二进制回读、420 条 exact fixtures、基线策略、回放、Web test/typecheck/build 与 Windows Tauri E2E。
