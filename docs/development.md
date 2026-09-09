# 开发与构建

## 环境

本仓库使用 Rust 2024 edition；工具链由 [rust-toolchain.toml](../rust-toolchain.toml) 固定为 Rust 1.96.0，包含 rustfmt 和 Clippy。前端使用 Node.js 24，与 [CI](../.github/workflows/ci.yml) 一致；依赖使用 `web/package-lock.json` 安装。

Windows 需要 Microsoft C++ Build Tools 和 WebView2。其他平台的系统依赖及 Android 工具链安装按 [Tauri 官方 prerequisites](https://v2.tauri.app/start/prerequisites/) 准备，仅安装本次实际要使用的平台环境。

在仓库根目录检查工具，再安装本工作树的前端依赖：

```powershell
rustc --version
node --version
cd web
npm ci
```

## 日常启动

在 `web` 执行 `npm run dev`。Tauri 会按 [tauri.conf.json](../web/src-tauri/tauri.conf.json) 启动 Vite，前端开发地址为 `127.0.0.1:1420`。关闭上一次占用该端口的开发实例后再启动新的实例。

`npm run dev:ui` 只启动 Vite；它不启动 Rust 游戏会话。无需为日常开发另建浏览器规则实现。

## 构建产物

下列 npm 命令均在 `web` 执行：

| 目标 | 命令 | 默认 Windows 输出 |
| --- | --- | --- |
| 前端资源 | `npm run build:ui` | `web/dist/`；命令包含 typecheck |
| 独立调试程序 | `npm run build:standalone:debug` | `target/debug/rfb-tauri.exe` |
| 优化后的独立程序 | `npm run build -- --no-bundle` | `target/release/rfb-tauri.exe` |
| Windows 安装包 | `npm run build` | `target/release/bundle/nsis/` |

这些 Tauri 构建会生成并嵌入前端资源。普通 `cargo build -p rfb-tauri` 的开发配置可能仍引用 Vite，不能用来证明已经产出可分发的独立程序。若设置了 `CARGO_TARGET_DIR`，输出位置随之改变。

构建成功说明产物生成成功；实际可玩性需要验证本批相关操作。把版本、提交、平台、操作和结果记入交付说明，不把一次启动扩展成全游戏验收。

## Android（需要时）

当前工程在 [web/src-tauri/gen/android](../web/src-tauri/gen/android/)，配置为 minSdk 24、compile/targetSdk 36。[构建脚本](../scripts/build-android.ps1)默认使用 NDK `29.0.13846066`，SDK 从参数、`ANDROID_HOME` 或本树 `.local/android-sdk` 取得。

按官方指南准备 JDK、SDK 与该版本 NDK 后，从仓库根目录执行：

```powershell
rustup target add aarch64-linux-android
./scripts/build-android.ps1 -SdkPath D:/Android/Sdk
```

脚本调用 `npm run android:build:debug`，目标为 ARM64 Debug APK；APK 位于 Android 工程的 `app/build/outputs/apk/` 下。此说明描述当前构建链，不代表当前提交已通过真机交互、生命周期或正式签名发布验收。

## 本地文件与常见问题

| 位置 / 现象 | 处理方式 |
| --- | --- |
| `target/`、`web/node_modules/`、`web/dist/` | 构建产物或依赖，可按用户要求重建；日常不主动清理其他工作树的目录 |
| `target/e2e/` | [E2E 构建脚本](../web/e2e/build.mjs)专用输出，不与普通桌面产物混用 |
| `test-results/` | 测试截图、日志与报告；可能是某次验收的证据 |
| `.local/` | 混有 SDK、工具输出和本地记录，不能当作纯缓存整目录删除 |
| `release/` | 用户保存的分发产物，不是构建缓存 |
| 正式内容 lock 不匹配导致 Rust 构建失败 | 先按[内容开发](content-development.md)完成 source/lock 同步；不要移除 build.rs 校验 |
| Vite 端口 1420 被占用 | 检查正在运行的开发任务，结束所属实例；不要随意终止无关进程 |

游戏存档使用 Tauri 的应用本地数据目录下 `saves/`，日志使用应用日志目录，诊断记录位于其 `diagnostics/`。实际位置由 [native_store / desktop_log_path](../web/src-tauri/src/lib.rs)决定，不在仓库缓存目录里。

不需要为了文档或普通规则修改构建桌面、Android。检查范围见[验证与契约](testing.md)。
