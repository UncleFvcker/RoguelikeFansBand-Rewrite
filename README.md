# RoguelikeFansBand Rewrite

用 Rust 重建 RoguelikeFansBand 的游戏规则，以 TypeScript、PixiJS 和 Tauri 2 提供原生客户端。保留回合制探索、ASCII 地图、装备鉴定与职业能力，并提供中文界面、鼠标操作和游戏内存档。

项目仍在开发，当前发布与试玩以 **Windows x64** 为主，仓库另有 Android 构建工程。已开放的功能与验收范围见[状态快照](docs/status.md)。

## 下载与运行

当前版本：**[v0.0.2](https://github.com/UncleFvcker/RoguelikeFansBand-Rewrite/releases/tag/v0.0.2)** · [下载 Windows x64 便携版 ZIP](https://github.com/UncleFvcker/RoguelikeFansBand-Rewrite/releases/download/v0.0.2/RoguelikeFansBand-Rewrite_0.0.2_windows-x64-portable.zip) · [所有发布版本](https://github.com/UncleFvcker/RoguelikeFansBand-Rewrite/releases)

1. 下载 ZIP，完整解压到可写目录。
2. 运行 `rfb-tauri.exe`，在标题页新建角色或继续游戏。
3. 游戏中按 `?` 查看操作手册，按 `=` 打开设置。

便携包仅包含 `rfb-tauri.exe` 和 `LICENSES/`，不提供 NSIS 安装包。游戏内容、界面与内置配色随 EXE 嵌入，无需安装 Rust、Node.js 或启动开发服务器；Windows 运行环境需要 WebView2。

角色存档保存在 EXE 旁的 `userdata/saves/`，关联角色档案保存在 `userdata/profile/`。备份或搬迁游戏时，请连同整个 `userdata/` 一起复制。全局偏好独立保存在 `%LOCALAPPDATA%\io.github.unclefvcker.rfb-rewrite\preferences.json`。具体操作见[游戏内存档](docs/builtin-save-system.md)。

## 现在可以做什么

- **角色与成长**：选择多种种族，以及战士、狂战士、法师、牧师、盗贼、食魔者等职业；按职业选择法术领域、学习施法和使用能力，预览尚未达到等级的能力。
- **探索与战斗**：在城镇、荒野和地牢间旅行，近战、射击、施法，完成任务、挑战首领，指挥宠物或骑乘。
- **装备与鉴定**：管理背包、装备与箭袋，交易、鉴定并逐步学习物品属性；名称与详情显示已知战斗数值。
- **自动行动**：长按移动、点击地图寻路、自动探索与按 Mogaminator 规则拾取；探索停止时显示具体原因。
- **地图与情报**：查看已知地图、记忆中的物品、怪物详情与发现档案；地形、怪物和物品按类型配色，unique 使用缓慢流动的彩虹渐变。
- **保存与记录**：游戏内保存、另存为、读取与备份恢复，查看消息、角色成绩，并使用回放工具。

v0.0.2 还改进了物品选择器与同格多物品拾取、消息堆叠、本层首领状态、目标血量、旧目标失效后的切换，以及鼠标选点。具体改动与实际检查见[发布说明](https://github.com/UncleFvcker/RoguelikeFansBand-Rewrite/releases/tag/v0.0.2)和[发布验证记录](docs/release-0.0.2-validation.md)。

当前尚未完成全部原版内容，商店布局等仍待改进。内容存在、规则实现、入口开放和实际试玩通过是不同状态；[状态快照](docs/status.md)与[后续工作](docs/next-work.md)记录各自范围。

## 常用操作

默认使用 **RFB Original** 预设，按键区分大小写；也可在设置中切换 **RFB Roguelike**。下表按默认预设说明。

| 操作 | 按键或鼠标 |
| --- | --- |
| 八方向移动 | 数字键／小键盘方向，支持长按 |
| 前往指定位置 | 左键点击地图地格 |
| 自动探索 | `Z` 或“自动探索”按钮 |
| 查看地格／怪物 | `l`，选中怪物后按 `r` 查看详情 |
| 选择目标 | `*`，再用键盘或鼠标点击确认 |
| 拾取 | `g`；脚下多件物品时弹出选择器 |
| 穿戴／脱下／丢弃 | `w`／`t`／`d` |
| 保存／保存并退出 | `Ctrl+S`／`Ctrl+X` |
| 帮助／知识／设置 | `?`／`~`／`=` |
| 取消或停止自动行动 | `Esc` |

瞄准模式中点击确认目标，查看模式中点击移动查看光标；连续移动时点击地图会先停止。完整键位与两套预设差异见[当前键盘操作](docs/keyboard-controls.md)，游戏内手册按当前预设显示。

## 从源码运行与构建

准备 Rust、Node.js 24 与 npm，以及 [Tauri 平台依赖](https://v2.tauri.app/start/prerequisites/)。Rust 版本由 [rust-toolchain.toml](rust-toolchain.toml) 固定。

在仓库根目录执行：

```powershell
cd web
npm ci
npm run dev
```

生成不需要 Vite 开发服务器的 Windows release 独立程序：

```powershell
# 在 web 目录
npm run build -- --no-bundle
```

默认输出为 `target/release/rfb-tauri.exe`。调试版使用 `npm run build:standalone:debug`，输出为 `target/debug/rfb-tauri.exe`。`npm run dev:ui` 仅启动前端开发服务器，完整游戏需要 Tauri 调用 Rust 核心。环境设置与常见问题见[开发与构建](docs/development.md)。

## 参与开发

| 我想做什么 | 从哪里开始 |
| --- | --- |
| 找到规则或界面的实现 | [架构与代码地图](docs/architecture.md) |
| 增加种族、职业、法术、物品或地点 | [内容开发](docs/content-development.md) |
| 选择测试、更新契约与生成文件 | [验证与契约](docs/testing.md) |
| 协调并行开发与交接 | [并行协作](docs/parallel-development.md) |
| 了解当前范围与尚未完成的工作 | [状态快照](docs/status.md)、[后续工作](docs/next-work.md) |

自动化助手的项目约定集中在 [AGENTS.md](AGENTS.md)。日常改动使用相关测试，验证通过后交付，不把完整 CI 或桌面套件作为每次编辑后的固定步骤。

## 仓库结构

```text
crates/       Rust 规则、内容、协议、存档、回放与导入工具
web/          TypeScript / PixiJS 客户端和 Tauri 外壳
packs/        正式内容源与内容锁
locales/      Fluent 文案
assets/       美术源文件；运行时资源在 web/public/
schemas/      机器可读格式，部分由 Rust 生成
tests/        当前契约 fixture
docs/         当前指南与独立的历史档案
```

完整导航见 [文档索引](docs/README.md)。旧 `design/` 材料已移入[历史档案](docs/archive/README.md)，原来的少数交接路径仅保留迁移链接。

## 来源与许可证

原项目：[RoguelikeFansBand-zh-CN](https://github.com/UncleFvcker/RoguelikeFansBand-zh-CN)。规则与内容开发使用另行安装的 RFB 仓库，通过 `master` Git 对象核对来源；设置方法见[内容开发](docs/content-development.md)。

原创软件使用 MPL-2.0，原创文档与创作内容使用 CC BY-SA 4.0。第三方及导入材料的权利不由本项目重新授予；以各文件声明、[许可证范围](LICENSES/README.md)和 [NOTICE](NOTICE) 为准。
