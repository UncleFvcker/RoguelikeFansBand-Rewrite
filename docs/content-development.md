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
3. 建立引用、参数、来源和所需文案；入口开放是单独的改动，不能只写 JSON 就报告玩家可用。
4. 内容变化更新包版本和 lock；实际类型变化才运行相应生成器。
5. 运行直接相关验证，检查通过后提交。详见[验证与契约](testing.md)。

原版内容、生成能力、角色实例和 UI 名称各有身份。先复用已存在的稳定 ID，避免一个怪物在召唤池和地牢生态中被重复导入、同一奖励在职业和任务中变成两个近似物品。

## 各方向在哪里改

| 内容 | 数据入口（正式包内） | 运行时与接入重点 |
| --- | --- | --- |
| 种族 / 职业 | `races/`、`classes/`、`skillSets/`、`builds/`、专属能力及 binding | 出生、属性/成长、实际专属行为、临时形态；菜单来源见 `PLAYTEST_*_IDS` |
| 领域 / 书本 | `abilities/`、`abilityPrograms/`、`playerAbilityBindings/`、`abilityBooks/`、实体书 `items/` | 源槽位、职业施法参数、学习与获取路径；施法使用现有 casting/targeting/effect |
| 物品 / Ego / 神器 / 装置 | `items/`、`affixes/`、`effectPrograms/` 及激活能力 | 生成、使用、装备、知识与实例生命周期；复用 loot 与已有物品规则 |
| 地牢 / 城镇 / 任务 | `worlds/middle-earth.json`、`towns/`、`townFacilities/`、`shops/`、`terrain/`、生态/掉落表 | 正式地点与 planned 地点区分，入口与楼层链、设施服务、奖励及必要保存恢复 |
| 怪物 / 守卫 | `actors/`、相关能力、生态选择与任务/地牢引用 | 先检查已导入身份；地牢专属内容由地点方向接入，法术召唤复用相同 actor |

数据结构在 [definitions/](../crates/rfb-content/src/definitions/)，引用和约束在 [validation/](../crates/rfb-content/src/validation/)，源格式差异在 [source/](../crates/rfb-content/src/source/)。Importer 的入口在 [main.rs](../crates/rfb-legacy-import/src/main.rs)，选择和适配记录位于包根目录的 `legacy-*.json`。仅运行对应内容的 audit/sync，不为一个小批次重导整个包。

已有八个高阶法师领域，先查[状态](status.md)，不要按旧待办重新实现。种族专属能力归种族职业；领域法术、通用物品归法术道具；任务和设施引用的物品定义与物品方向共享。实际冲突按[并行协作](parallel-development.md)处理。

## 内容锁与生成文件

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
