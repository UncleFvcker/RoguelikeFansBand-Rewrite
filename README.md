# RoguelikeFansBand Rewrite

用 Rust 重建 RoguelikeFansBand 的游戏规则，以 TypeScript、PixiJS 和 Tauri 2 提供原生客户端。项目仍在开发：已有角色创建、战斗、学习施法、装备与物品、城镇交易、荒野和地牢探索，以及存档和回放。

当前开发和试玩以 Windows 桌面为主，仓库另有 Android 构建工程。前端开发服务器用于开发与检查；完整游戏通过 Tauri 调用 Rust 核心。

## 开始运行

准备 Rust、Node.js 24 与 npm，以及 [Tauri 平台依赖](https://v2.tauri.app/start/prerequisites/)。Rust 版本由 [rust-toolchain.toml](rust-toolchain.toml) 固定。

在仓库根目录执行：

```powershell
cd web
npm ci
npm run dev
```

生成不需要 Vite 开发服务器的 Windows 调试版：

```powershell
# 在 web 目录
npm run build:standalone:debug
```

默认输出为 `target/debug/rfb-tauri.exe`。发布构建、环境设置和常见问题见 [开发与构建](docs/development.md)。

## 现在可以做什么

新游戏目前提供战士、死亡领域高阶法师、弓箭手、死亡领域圣骑士、骑兵和狙击手。其他七个高阶法师领域已有内容与规则路径，但还没有开放新游戏入口。

内容存在、规则实现、入口开放和实际试玩通过是不同的状态。[状态快照](docs/status.md)列出代码依据及验收范围；[后续工作](docs/next-work.md)只列仍需核对或推进的具体方向。测试通过不等于全部原版内容已经完成。

## 参与开发

| 我想做什么 | 从哪里开始 |
| --- | --- |
| 找到规则或界面的实现 | [架构与代码地图](docs/architecture.md) |
| 增加种族、职业、法术、物品或地点 | [内容开发](docs/content-development.md) |
| 选择测试、更新契约与生成文件 | [验证与契约](docs/testing.md) |
| 参与当前三个工作树的开发 | [并行协作](docs/parallel-development.md) |
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
