# RoguelikeFansBand Rewrite

RoguelikeFansBand 的下一代重写版：以确定性的 Rust 规则核心为基础，通过 TypeScript、PixiJS 与 Tauri 2 提供桌面和 Android 客户端。

项目不直接移植旧版 C 代码，而是先用协议、内容格式和精确行为契约固定规则，再逐步重建玩法。浏览器、PWA 与 WASM 不是目标运行环境。

## 当前状态

当前状态按“内容已定义、规则已实现、玩家入口已开放、实际验收通过”分别记录，统一见 [当前状态](design/current-status.md)（2026-09-09 核对）。

- **内容与规则**：已接入荒野、城镇、地下城、角色、物品、怪物等可玩切片。高阶法师八领域均有四册内容及对应规则路径；定义数量和规则测试不等于玩家入口或全量验收。
- **玩家入口**：新游戏开放战士、死亡领域高阶法师、弓箭手、固定死亡领域圣骑士、骑兵和狙击手，共 6 个构筑、42 个种族。高阶法师其余七领域尚未开放新游戏入口。
- **实际验收**：2026-09-09 同源桌面自动验收通过战士和死亡领域高阶法师的新游戏、菜单、装备、学习施法及保存恢复相关流程；正式 standalone EXE 完成构建与启动核验。未宣称全职业、全领域、完整通关或 Android 验收通过。详见 [版本验收记录](design/playable-release-20260909.md)。
- **版本与内容统计**：集中维护在 [当前状态基线](design/current-status.md#当前基线与内容定义)，精确版本与哈希以 [pack.json](packs/rfb-demo-original/pack.json) 和 [content.lock.json](packs/rfb-demo-original/content.lock.json) 为准。

## 快速开始

需要 Rust 工具链、Node.js 与 npm，以及当前平台所需的 Tauri 依赖。

```powershell
cd web
npm ci
npm run dev
```

构建无需 Vite 开发服务器的独立调试版：

```powershell
cd web
npm run build:standalone:debug
```

生成的 Windows 可执行文件位于 `target/debug/rfb-tauri.exe`。普通 Cargo 构建仍可能依赖 Vite 开发服务器。

## 技术与结构

- Rust：确定性规则、内容编译、存档、回放和契约测试。
- TypeScript + Vite：客户端状态、交互与构建。
- PixiJS：地图和游戏画面渲染。
- Tauri 2：原生应用外壳与 IPC。
- Fluent：本地化。

| 路径 | 职责 |
| --- | --- |
| `crates/rfb-core` | 游戏规则、运行时状态与确定性模拟 |
| `crates/rfb-protocol` | IPC DTO、TypeScript 绑定与协议 Schema |
| `crates/rfb-content` | 内容包加载、编译与验证 |
| `crates/rfb-contract` | 精确行为 fixture 与回归基线 |
| `crates/rfb-save`、`crates/rfb-replay` | 存档容器与回放 |
| `crates/rfb-legacy-*` | 旧版数据探测与导入工具 |
| `web` | TypeScript/PixiJS 前端 |
| `web/src-tauri` | Tauri 原生后端 |
| `packs/rfb-demo-original` | 当前演示内容包 |
| `design`、`tests/fixtures/active` | 设计记录与有效契约样例 |

## 关键文档

- [系统实现路线图](design/rfb-system-implementation-roadmap.md)
- [待实现清单](design/pending-implementation.md)
- [发布垂直切片](design/release-vertical-slice.md)
- [协议 v1](design/protocol-v1.md)
- [内容格式 v1](design/content-format-v1.md)
- [存档格式 v1](design/save-format-v1.md)
- [确定性模拟](design/deterministic-simulation.md)
- [基线更新策略](design/baseline-update-policy.md)
- [物体列表与本地旅行](design/object-list-o1-item-discovery.md)
- [物品集成现状](design/phase-19-legacy-item-integration.md)
- [怪物接入与对话交接手册](design/warrens-monster-integration-handoff.md)
- [怪物机制待办](design/warrens-monster-mechanism-backlog.md)
- [荒野 W5 扩展](design/wilderness-w5-original-extensions.md)
- [荒野卷动坐标模型](design/contract-v231-wilderness-scroll-coordinate-model.md)
- [荒野分代地形与缓存](design/contract-v232-wilderness-evolving-terrain-cache.md)
- [荒野区块卷动与状态迁移](design/contract-v233-wilderness-chunk-scrolling.md)
- [荒野暴露条带怪物生成](design/contract-v234-wilderness-strip-monster-population.md)
- [荒野卷动客户端状态同步](design/contract-v235-wilderness-client-translation.md)
- [荒野卷动城镇边界](design/contract-v236-wilderness-town-boundary.md)
- [连续荒野中的可变尺寸城镇](design/contract-v237-wilderness-embedded-towns.md)

历史契约与专题设计均保留在 [`design/`](design/) 中。

## 验证

日常开发按改动范围选择相关检查：

```powershell
cargo fmt --all -- --check
cargo test -p rfb-core
cargo test -p rfb-content
cargo test -p rfb-contract
cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings

cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original

cd web
npm test
npm run typecheck
npm run build:ui
```

协议生成会同步 `web/src/protocol.ts` 与 `schemas/protocol-v1.schema.json`。完整 workspace、桌面 E2E 和 Android 构建仅在相关改动或里程碑验收时运行；细节见 [桌面 E2E](design/tauri-desktop-e2e.md) 与 [Android 目标](design/android-target.md)。

契约 fixture 的 `assertions.finalState` 只填写场景需要检查的字段，必须保留完整 `stateHash`。对象按已填写的字段递归比较；空对象表示集合必须为空，数组精确比较内容和顺序。商店、背包、装备、实体、任务、物品知识、武器熟练度和材料等有稳定 ID 的集合使用 ID 作为对象键。事件、错误和存档往返哈希仍精确比较。`refresh` 会保留已填写的字段范围；新增 fixture 可先用 `observe` 生成完整观察值，再裁剪到场景目标。完整投影由专门的核心测试覆盖。

## 旧版来源

原项目：[UncleFvcker/RoguelikeFansBand-zh-CN](https://github.com/UncleFvcker/RoguelikeFansBand-zh-CN)。

新增规则与内容以本地 `D:/codex/Frogcomposband/master` 仓库的 `master` Git ref 为权威来源，并通过 Git 对象读取；不要依赖该仓库当前检出的分支或工作树。环境变量 `RFB_LEGACY_SOURCE` 可指定旧版仓库位置。历史契约和旧存档样例只有在明确固定提交时才沿用旧 ref。

## 许可证

- 代码、测试和 Schema：MPL-2.0。
- 项目自有文档、游戏数据和美术：CC BY-SA 4.0。
- 第三方依赖和导入资产继续遵循各自许可证，旧版内容不会因导入而被重新授权。

详见 [`LICENSES/README.md`](LICENSES/README.md) 与 [`NOTICE`](NOTICE)。
