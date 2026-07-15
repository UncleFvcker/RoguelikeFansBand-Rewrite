# RoguelikeFansBand Rewrite

RoguelikeFansBand 的新一代重构工程。

本仓库不直接复制旧 C 工程，而是以稳定协议和行为测试为边界，逐步重新实现游戏核心与前端。

## 技术方向

- Rust：游戏规则、数据模型、随机数、存档、AI 与原生核心
- TypeScript + Vite：Tauri WebView 界面和开发工具
- PixiJS：地图、tileset、光照与动画渲染
- Tauri 2 IPC：TypeScript UI 与原生 Rust 核心通信
- Tauri 2：Windows、Linux、macOS 和 Android 封装
- Fluent：英文/简体中文本地化

当前不以浏览器/PWA 为发布目标，也不维护 WASM 核心。UI 通过 `CoreTransport` 连接 `TauriNativeTransport`；未来如确有网页需求，再单独增加 WASM 适配器。

## 设计文档

- [Rust/Tauri 重构计划](design/html-rewrite-plan.md)
- [旧版行为基准与差分测试](design/legacy-behavior-baseline.md)
- [Contract 基准更新与差异豁免政策](design/baseline-update-policy.md)
- [核心协议 v1](design/protocol-v1.md)
- [确定性模拟、随机数与回放](design/deterministic-simulation.md)
- [内容数据格式 v1](design/content-format-v1.md)
- [Tileset manifest 与资源回退 v1](design/tileset-format-v1.md)
- [新存档格式 v1](design/save-format-v1.md)
- [授权、版权与素材迁移审计](design/licensing-and-assets.md)
- [本地化与中文文本重构计划](design/localization-rewrite-plan.md)

原创规则契约位于 [`tests/fixtures/contract-v1/scenarios`](tests/fixtures/contract-v1/scenarios)，由 `rfb-contract` 在所有平台运行。

确定性命令回放由 [`rfb-replay`](crates/rfb-replay) 提供：正式 `.rfbreplay` 使用带 SHA-256 校验的 MessagePack 容器，JSON 仅用于调试。

## 原项目

旧版 RFB 源码和当前可玩版本继续保留在：

[UncleFvcker/RoguelikeFansBand-zh-CN](https://github.com/UncleFvcker/RoguelikeFansBand-zh-CN)

旧项目在重构期间只作为规则行为、平台表现和旧存档格式的本地参考实现。

旧版内容不会复制进本仓库或新游戏发行包。开发工具通过本地环境变量 `RFB_LEGACY_SOURCE` 只读访问旧仓库，并固定读取 `v1.3.0.7`；默认开发路径见 [`.env.example`](.env.example)。新游戏内容、文本和素材均单独创作。

## 许可证

- 原创 Rust/TypeScript 代码、工具、测试和 Schema：`MPL-2.0`；
- 原创文档、游戏数据和美术素材：`CC BY-SA 4.0`；
- 第三方内容：保留各自许可证；
- 旧 RFB/FrogComposband/Angband 内容不在本仓库中，也不由上述许可证重新授权。

完整适用范围见 [LICENSES/README.md](LICENSES/README.md) 和 [NOTICE](NOTICE)。

## 当前阶段

Tauri 2 Windows 原生垂直切片已经建立：`TauriNativeTransport` 直接调用 Rust 核心，移动、基础战斗、三套键位预设、`.rfbsave` 存档和 `.rfbreplay` 诊断回放均已接入。旧 `rfb-wasm`、Web Worker、wasm-pack 和 wasm32 构建目标已经从 workspace、前端和 CI 删除。

### 本地验证

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p rfb-contract
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original

cd web
npm ci
npm test
npm run build -- --no-bundle
# 启动可玩开发版：npm run dev
```

Rust 是 CoreTransport DTO 的唯一权威来源。修改 `rfb-protocol` 后运行：

```powershell
cargo run -p rfb-protocol --features bindings --bin generate-bindings
```

该命令更新 `web/src/protocol.ts` 和 `schemas/protocol-v1.schema.json`；CI 使用 `--check` 拒绝未同步的生成文件。

验证或编译原创内容包：

```powershell
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-content --bin rfb-contentc -- compile packs/rfb-demo-original target/generated/rfb-demo-original.rfbcontent
cargo run -p rfb-content --features schemas --bin generate-content-schemas
```

内容编译器会严格解析 JSON、校验稳定 ID/引用/范围，规范化排序后输出带 SHA-256 校验的 MessagePack 容器。首个原创包的固定 content hash 记录在 `packs/rfb-demo-original/content.lock.json`。

如需生成本地旧版参考 manifest：

```powershell
$env:RFB_LEGACY_SOURCE="D:/codex/Frogcomposband/master"
$env:RFB_LEGACY_REF="v1.3.0.7"
$env:RFB_LEGACY_COMMIT="191f48c3fd1cdbc81a3d3395a88cd6758402b4d9"
cargo run -p rfb-legacy-probe
```

登记本地旧存档样本时显式传入至少 3 个旧仓库内的文件路径：

```powershell
cargo run -p rfb-legacy-probe -- catalog-saves <旧存档1> <旧存档2> <旧存档3>
```

工具只把中性命名副本、SHA-256、四字节版本头和本地清单写入被 Git 忽略的 `.local/legacy-baseline/`。当前机器已经准备两份 1.3.0.7 样本和一份 1.2.0.6 迁移样本。

解析旧存档的稳定前缀并建立本地字段断言：

```powershell
cargo run -p rfb-legacy-import -- inspect-prefix .local/legacy-baseline/saves/legacy-save-01.bin
cargo run -p rfb-legacy-import -- record-catalog .local/legacy-baseline/save-samples.json
cargo run -p rfb-legacy-import -- verify-catalog .local/legacy-baseline/save-samples.json
```

`rfb-legacy-import` 当前只读取不依赖旧 C 结构体内存布局的 409 字节稳定前缀，包括版本、保存元数据、63 项 RNG 状态和选项位。生成的 `parsed-save-samples.json` 仍位于 `.local/`，不会进入 Git；`record-catalog` 拒绝覆盖已有基线。

快照规范化和 hash：

```powershell
cargo run -p rfb-contract -- normalize-snapshot <snapshot.json>
cargo run -p rfb-contract -- hash-snapshot <snapshot.json>
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v1/baseline-policy.json
```

首批 20 个原创 contract fixtures、自动协议生成、原创内容包、ASCII glyph atlas、图片 tileset manifest、缺失资源回退和 Windows Tauri 端到端测试已经建立。桌面 E2E 可用以下命令运行：

```powershell
cd web
npm run e2e
```

测试覆盖地图局部更新、Canvas/HTML 消息分层、存档导出与恢复、回放导出和 tileset 热切换；失败时会在仓库根目录的 `test-results/` 生成截图和日志。

Tauri Android ARM64 Debug APK 构建链也已经建立，Windows 本地可运行：

```powershell
.\scripts\build-android.ps1 -Proxy http://127.0.0.1:7897
```

Android 与 Windows 使用同一个 Rust 核心和 Tauri Commands。详细依赖、产物位置和当前尚未完成的真机验证见 [Tauri Android 原生目标](design/android-target.md)。
