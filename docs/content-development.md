# 内容开发

## 先定位事实与现有实现

本项目以独立 Rust 实现承载 RFB 行为。新增规则与内容读取本地 `D:/codex/Frogcomposband/master` 仓库的 `master` Git ref；仓库目录名与 Git ref 是两件事。只读 Git 对象，不依赖该仓库当前检出的文件。

```powershell
git -C D:/codex/Frogcomposband/master rev-parse master
git -C D:/codex/Frogcomposband/master show master:lib/edit/d_info.txt
git -C D:/codex/Frogcomposband/master grep -n '目标符号' master -- src lib
```

用到导入工具时，在当前终端设置 `$env:RFB_LEGACY_SOURCE = 'D:/codex/Frogcomposband/master'`。`.env.example` 中的旧 ref/commit 是历史存档探测配置，不是新内容的默认来源。

记录本批实际源码提交、source index/符号和必要映射即可；不用每次另建全仓审计表。原版说明与实际执行不一致时追踪真实分支并记录差异。中文名称逐字取自权威中文表/源字符串，未找到就列 unresolved，不翻译补齐。

## 一批内容的最短闭环

1. 搜索正式包的稳定 ID 与当前运行时，明确这次缺的是数据、行为还是玩家入口。
2. 复用已有机制，只补当前内容确实缺失的表达或执行路径。不要为下一职业、下一领域预造配置与抽象。
3. 建立引用、参数、来源和所需文案；入口开放是单独的改动，不能只写 JSON 就报告玩家可用。新增职业或领域 Build 执行下文的[生成接入闭环](#职业与领域-build-的生成接入)。
4. 内容变化更新包版本和 lock；实际类型变化才运行相应生成器。
5. 运行直接相关验证，检查通过后提交。详见[验证与契约](testing.md)。

原版内容、生成能力、角色实例和 UI 名称各有身份。先复用已存在的稳定 ID，避免一个怪物在召唤池和地牢生态中被重复导入、同一奖励在职业和任务中变成两个近似物品。

## 各方向在哪里改

| 内容 | 数据入口（正式包内） | 运行时与接入重点 |
| --- | --- | --- |
| 种族 / 职业 | `races/`、`classes/`、`skillSets/`、`builds/`、专属能力及 binding | 出生、属性/成长、实际专属行为、临时形态；实际菜单来源见 [character-creation.ts](../web/src/character-creation.ts)，新 Build 同步生成适用性审计 |
| 领域 / 书本 | `abilities/`、`abilityPrograms/`、`playerAbilityBindings/`、`abilityBooks/`、实体书 `items/` | 源槽位、职业施法参数、学习与获取路径；施法使用现有 casting/targeting/effect |
| 物品 / Ego / 神器 / 装置 | `items/`、`affixes/`、`randomArtifacts/`、`effectPrograms/` 及激活能力 | 生成、使用、装备、知识与实例生命周期；复用 loot 与已有物品规则 |
| 地牢 / 城镇 / 任务 | `worlds/middle-earth.json`、`towns/`、`townFacilities/`、`shops/`、`terrain/`、生态/掉落表 | 正式地点与 planned 地点区分，入口与楼层链、设施服务、奖励及必要保存恢复 |
| 怪物 / 守卫 | `actors/`、相关能力、生态选择与任务/地牢引用 | 先检查已导入身份；地牢专属内容由地点方向接入，法术召唤复用相同 actor |

数据结构在 [definitions/](../crates/rfb-content/src/definitions/)，引用和约束在 [validation/](../crates/rfb-content/src/validation/)，源格式差异在 [source/](../crates/rfb-content/src/source/)。Importer 的入口在 [main.rs](../crates/rfb-legacy-import/src/main.rs)，选择和适配记录位于包根目录的 `legacy-*.json`。仅运行对应内容的 audit/sync，不为一个小批次重导整个包。

剩余物品按[覆盖计划](remaining-item-coverage-plan.md)和[逐项核对清单](../design/remaining-item-coverage-review.json)分批接入。基础 kind、固定神器和装置效果分别计数；清单分类不能代替行为验收。
设置 `RFB_LEGACY_SOURCE` 后，可用 `cargo run -p rfb-legacy-import -- audit-demo-items packs/rfb-demo-original/legacy-item-selection.json packs/rfb-demo-original/legacy-item-adaptations.json - packs/rfb-demo-original/items` 只读盘点当前来源映射与 importer 阻塞。`-` 明确不检查历史 P3 进度；需要核对历史计划时仍传原 plan 路径。正式物品自身的 `rfbBaseKind` 参与计数，不要求为同一身份重复补旧选择表；未覆盖消费者的 mechanics-ready 不得当作可玩完成。

已有九个高阶法师领域，先查[状态](status.md)，不要按旧待办重新实现。种族专属能力归种族职业；领域法术、通用物品归法术道具；任务和设施引用的物品定义与物品方向共享。实际冲突按[并行协作](parallel-development.md)处理。

## 职业与领域 Build 的生成接入

后续职业或领域接入计划必须把以下闭环写入来源核对、入口开放和验收步骤：**新增 Build → 核对来源 → 记录差异或无差异 → 完成实现与消费者证据 → 通过 CI 检查**。
已有 Class 新开放另一领域 Build 也要单独记录。详细依据和已完成的接入工作见[生成接入计划](class-generation-integration-plan.md)，当前范围见[状态页](status.md)。

1. 从[创角目录](../web/src/character-creation.ts)取得实际 Build，读取正式 `builds/`、`classes/` 的 Class、领域和装备能力。核对原版 `master` 的 `ego.c`、`object2.c`、`artifact.c` 及关联消费者，记录实际来源提交；不仅查直接职业判断，也检查身体槽位、骑乘、书本发现数和成品可用性。
2. 在[审计输入](../design/generation-build-applicability.json)补充该 Build 的五个范围：基础分配/Tailored、Ego/负向生成、随机神器、固定神器/奖励、使用/保存。复用共同条件 ID 与实现/测试引用，条件只在这里维护，不另建职业生成允许列表。
3. `implemented` 需要实际入口、实现和验证引用；`no-special-difference` 需要来源及间接消费者的核对理由。当前可达范围尚未完成时，保留 `deferred` 的依赖及与 Build 双向关联的 gap，不标为完成。未开放卷轴、缺失身份或内容单列；`deferred-unavailable-build` 必须明确 `unavailableClassIds` 或 `unavailableRaceIds`，身份开放后重新核对这些依赖。
4. 只补缺失的实际行为与证据。使用真实新游戏 Build 验证生成物的装备、使用或按规则拒绝、保存恢复和必要的后续 RNG；复用已有共同覆盖，概率分支用可复现边界，不伪造 Class ID 代替可玩验收。自然/主题/卷轴模式分开记录，职业不能使用某物品本身不是从普通生成池删除它的依据。
5. 更新输入后运行完整来源审计，重生成[生成矩阵](../design/ego-contract-audit.json)，再运行只读检查并随该 Build 的改动提交。不要手改报告使其通过，也不要只补一个完成标签来关闭 gap。

仓库根目录使用 Node 24：

```powershell
# 输入或来源审查结果变化后：需要 Rust 和原版 Git 仓库
node scripts/audit-egos.mjs D:/codex/Frogcomposband/master
# 日常检查与 CI 使用同一只读入口：不启动 Rust、不读取原版仓库、不写文件
node scripts/audit-egos.mjs --check-applicability
```

来源提交变化时先复核差异，再更新审计依据，不仅替换提交号。检查脚本自身发生变化时，运行 `node --test scripts/generation-applicability.test.mjs`；现有前端 CI 同时运行只读检查和这些工具测试。
缺失 Build、悬空引用、身份开放矛盾、报告过期或与 gap 矛盾的完成标记都会失败。CI 通过只证明记录和入口一致，引用存在也不表示游戏测试已经执行；交付须写明实际检查结果、开放范围和剩余依赖。
有记录的缺口是否允许按受限范围开放，由该职业/领域的明确需求决定，不能由脚本自动豁免或禁止。负责人和合并检查见[并行协作](parallel-development.md#职业与领域生成审计的交接)。

## 内容锁与生成文件

基础分配的定向同步只更新当前正式物品的 source kind 身份、权威中文显示名、基础分配行和主题引用，保留现有物品效果/装置适配。中文词干来自 `kind_name_zh.inc`；药水、卷轴和蘑菇类别后缀沿用 `flavor.c` 的已知无外观显示格式。基础池按层级、source kind 和原分配行顺序排列，零权重及重复行保留。覆盖报告位于包根目录 `legacy-base-allocation-audit.json`，不属于运行时内容。

```powershell
$env:RFB_LEGACY_SOURCE = 'D:/codex/Frogcomposband/master'
cargo run -p rfb-legacy-import -- sync-demo-base-allocation packs/rfb-demo-original
```

随机神器数据单独同步。第一条命令读取 `master` 的名字文件及激活表；第二条通过原版 C 估值补齐激活价值，需要本机 C 编译器。两步完成后再更新包版本与 lock，不重导其他内容。

```powershell
$env:RFB_LEGACY_SOURCE = 'D:/codex/Frogcomposband/master'
cargo run -p rfb-legacy-import -- sync-demo-random-artifacts packs/rfb-demo-original
python scripts/generate-random-artifact-reference.py D:/codex/Frogcomposband/master
```

[pack.json](../packs/rfb-demo-original/pack.json)声明版本及 contentRoots；[content.lock.json](../packs/rfb-demo-original/content.lock.json)记录 pack ID、版本和编译 hash。包目录里存在一个子目录，不代表它已经进入 contentRoots。

在仓库根目录执行：

```powershell
# 修改内容和 pack version 后，取得真实编译摘要
cargo run -p rfb-content --bin rfb-contentc -- inspect-source packs/rfb-demo-original
```

将摘要中的 `packId`、`packVersion`、`contentHash` 同步到现有 lock，保持 lock 的 schemaVersion。然后：

```powershell
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
```

`inspect-source` 不写锁，`verify-source` 验证锁；没有 `write-lock` 子命令。不要手工估算 hash，也不要把另一个分支的 hash 拷贝过来冒充合并后的内容。

| 真正改变的定义 | 生成命令 |
| --- | --- |
| Rust 协议 DTO | `cargo run -p rfb-protocol --features bindings --bin generate-bindings` |
| Rust 内容类型 | `cargo run -p rfb-content --features schemas --bin generate-content-schemas` |

生成后给同一命令追加 `-- --check` 检查。协议生成覆盖 `web/src/protocol.ts` 和 `schemas/protocol-v1.schema.json`；内容生成覆盖 `schemas/content-v1/`。纯 JSON 参数或文案变更不需要重复生成无变化的类型文件。包外 UI 文案不改变内容包时，不单独升 pack 版本。

## 文案与美术接入

Fluent 的 `content.ftl` 承载内容键，`ui.ftl` 承载界面文本。中英文键与插值参数保持一致；中文 RFB 名称来自原版，不能从英文自动翻译。格式和术语可参考 [中文风格资料](../locales/glossary/style-guide.zh-CN.md)，但事实名称仍以实际来源为准。

图像资源使用 [tileset manifest](../schemas/tileset-v1.schema.json) 的语义 ID 映射。已有三套运行时目录 `ascii-default`、`image-demo`、`rfb-pixel-28`；新增专属 actor 时检查需要的映射，不为普通新种族复制玩家图集。美术源与导出关系见 [STYLE.md](../assets/tilesets/rfb-pixel-28/STYLE.md)。现有 glyph fallback 是渲染格式的一部分，不因反对泛化设计就擅自删除。

## 交付的边界

一个批次写清新增/修改的实际行为、稳定 ID、来源、相关检查与剩余缺口即可。没有新增状态，就不自动创建迁移器和一套 save/replay 测试；新增权威状态则必须进入现有初始化、保存恢复与哈希流程，不能把未接入的半成品当作完成。

来源记录不自动授予再分发权利。保留文件声明和已有许可材料；历史授权结论或项目原创许可证不能替代导入内容自己的权利依据。
