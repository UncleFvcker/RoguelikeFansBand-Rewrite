# v0.0.2 便携版发布验证

2026-09-15。用户授权提交、推送并发布 `v0.0.2`，随后授权同版本重新发布，增加新版标题页和三维地牢入口。初次发布整合 v0.0.1 之后的装备属性展示、消息堆叠、首领与目标血量、瞄准记忆、怪物详情、ASCII 配色、陷阱与物品共格、物品选择器、探索停止原因和鼠标地图操作。

## 同版本重新发布

- 新增 RoguelikeFansBand 大标题、纵向菜单、最近可继续存档摘要、键盘导航和三维地牢入口 shader；修复按钮 Enter 被游戏命令菜单拦截。
- 本批沿用已验证的普通 standalone，不重复构建或跑全量回归。标题页相关单测累计 164 项通过；3D 最终版本类型检查、构建和 10 组普通 EXE 专项验收通过，涵盖四种尺寸、GPU 像素预算、菜单、创角／保存／继续、动画暂停与上下文丢失回退。WebGPU 仅检查 WGSL 编译，实际场景使用 WebGL。详情见[标题页记录](title-screen-redesign-plan.md)。
- 发布 EXE 与最终桌面验收副本 SHA-256 一致：`f6e9ee658569d79aabba7a7a282f68418f737b3113a8f05bc43698db7d4312a3`。应用版本仍为 `0.0.2`；不改协议、存档、内容版本。
- 更新 `v0.0.2` 标签与原 portable ZIP 附件，保持 ZIP 根目录仅含 `rfb-tauri.exe` 和 `LICENSES/`；来源说明指向重新发布的提交，不含用户数据或 NSIS。

## 版本与产物

- Cargo workspace、npm 和 Tauri 应用版本统一为 `0.0.2`。
- 协议 `1.299`，save payload `45`，State Hash Schema `150`；协议绑定和 JSON Schema 同步。
- 内容包与 lock 保持 `1.485.0`，hash 为 `b5d6cd5b28bba3ba254179b89b8c95d087a95ce910805abb3d97ed901b234b0a`。`verify-source` 与 216 种创角构建的 ego applicability 审计通过。
- Windows x64 使用 `npm run tauri -- build --no-bundle` 构建普通 release standalone，包含类型检查与 Vite 构建；EXE 的 FileVersion／ProductVersion 均为 `0.0.2`。Tauri bundle 已关闭，不创建 NSIS。
- ZIP 仅含 `rfb-tauri.exe` 和 `LICENSES/`，不含用户数据、测试产物或开发文件。内容、界面与 tileset 嵌入 EXE；运行后在 EXE 旁创建 `userdata/`，全局偏好仍使用应用数据目录。
- 初次发布 EXE SHA-256：`8e858a04df2e991559e84b585f99cec3dcbe95b7cff737d1c8be218574e00d24`，由上方重新发布产物替换。

## 初次发布验证结果

- 前端全量 490 项通过。
- Rust 全量回归通过：Core 2,187 项（另 18 项显式忽略）；Content 183、Contract 库及集成 17（另 1 项显式忽略）、Legacy import 223、Legacy probe 2、Localization 40、Protocol 7、Replay 13、Save 2、Tauri 36 项。首轮在契约版本常量检查处停止，修复后继续执行 workspace，仅排除首轮已经通过的 Content，不重复已通过的整轮测试。
- `cargo clippy --workspace --all-targets -- -D warnings`、Rust 格式和差异空白检查通过。
- 普通 release EXE 在独立安装目录中新建人类战士，通过 WebView2 CDP 操作真实界面：丢弃物品的列表／数量确认、同格多物品按 `g` 选择且只拾取指定实例、鼠标确认地图目标不消耗回合、鼠标点击相邻地格后到达目的地、原生保存。没有捕获到运行时异常，未修改现有全局偏好。
- 本地证据位于忽略目录 `test-results/release-0.0.2/`，包括各项日志、`smoke-report.json`、选择器截图和最终游戏截图。桌面脚本仅观察 IPC 响应；调试连接与数量确认步骤修正后通过，未因此改变生产逻辑。

## 契约差异

`contract-v346` 的 26 条 active 契约全部通过。相对 v0.0.1 的 `contract-v344`：两个战斗 fixture 增加 `attackTarget`；默认 `operationOptions.defaultTarget` 改为旧目标优先、否则最近敌人，因其参与既有状态哈希而更新 26 个 fixture 的 41 处哈希。RNG 和其余状态断言保持，State Hash Schema 仍为 `150`。

发布回归发现库内 `ACTIVE_BASELINE` 仍为 `contract-v343`，已同步到现行 policy 的 `contract-v346`。修复后 policy 测试及契约验证通过，没有在本次发布验证中改写 fixture 预期。

## 范围边界

桌面冒烟覆盖上述交互，不替代长期游玩、所有职业／地牢场景或 Android 验收。装备数值继续受鉴定知识约束；商店布局和商店中的装备摘要仍待原计划 F4/F5 后续工作。
