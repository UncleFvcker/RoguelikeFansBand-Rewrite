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
- [核心协议 v1](design/protocol-v1.md)
- [确定性模拟、随机数与回放](design/deterministic-simulation.md)
- [内容数据格式 v1](design/content-format-v1.md)
- [新存档格式 v1](design/save-format-v1.md)
- [授权、版权与素材迁移审计](design/licensing-and-assets.md)
- [本地化与中文文本重构计划](design/localization-rewrite-plan.md)

原创规则契约位于 [`tests/fixtures/contract-v1/scenarios`](tests/fixtures/contract-v1/scenarios)，由 `rfb-contract` 在所有平台运行。

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

Tauri 2 Windows 原生垂直切片已经建立：`TauriNativeTransport` 直接调用 Rust 核心，移动、基础战斗、三套键位预设和 `.rfbsave` 存档均已迁移。旧 `rfb-wasm`、Web Worker、wasm-pack 和 wasm32 构建目标已经从 workspace、前端和 CI 删除。

### 本地验证

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p rfb-contract

cd web
npm ci
npm run build -- --no-bundle
# 启动可玩开发版：npm run dev
```

如需生成本地旧版参考 manifest：

```powershell
$env:RFB_LEGACY_SOURCE="D:/codex/Frogcomposband/master"
$env:RFB_LEGACY_REF="v1.3.0.7"
$env:RFB_LEGACY_COMMIT="191f48c3fd1cdbc81a3d3395a88cd6758402b4d9"
cargo run -p rfb-legacy-probe
```

输出只写入被 Git 忽略的 `.local/legacy-baseline/`。首批 20 个原创 contract fixtures 已建立；下一步是增加回放文件 v1、每 100 命令 state hash 检查点和本地旧存档导入样本。
