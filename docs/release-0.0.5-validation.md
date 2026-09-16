# v0.0.5 便携版发布验证

2026-09-16。整合创角数值与成长明细、Tab 接近敌人并近战、物品栏操作返回规则、地图操作栏精简、游戏键位设置入口及等级区间经验条。

## 版本与产物

- Cargo workspace、npm、Tauri、EXE FileVersion／ProductVersion 均为 `0.0.5`；锁文件仅同步项目版本，第三方依赖未变。
- 协议 `1.302`，生成的 TypeScript／Schema 已同步；save payload `46`、State Hash Schema `151`、内容包及 lock `1.486.0` 不变，未刷新契约基线。
- `npm run build -- --no-bundle` 完成普通 Windows x64 Tauri standalone 优化构建，产物为 `target/release/rfb-tauri.exe`；不包含 WebDriver 专用接口，不生成 NSIS。
- EXE SHA-256：`7c3fab12896e5f5b10a72e8ee4a8c0c7c226564954eb35f09149f12aee11ae9c`。
- 便携 ZIP 根目录仅含 `rfb-tauri.exe` 和 `LICENSES/`，不包含 userdata、个人偏好、测试资料或调试符号；源码出处指向 v0.0.5 及本次提交。

## 已执行验证

- 沿用本批刚完成的 331 项相关前端测试、40 项核心测试、8 项协议测试及 26 条共享契约，涵盖创角、属性与职业被动、连续行动、输入、物品栏、设置和经验条。没有运行全量测试。
- 修正创角测试的默认行为偏好构造；Tab 用例改用普通心智怪物，避免将战争熊的怪异心智误设为稳定可感应目标。原断言保留，失败用例复验通过。
- 协议生成一致性、修改 Rust 文件格式和差异检查通过。正式发布构建重新完成 TypeScript 检查、Vite 打包和 Rust 优化链接；保留已有 Vite 大文件提示。
- 优化后的普通 EXE 在独立安装／WebView 目录创建全新人类战士：验证设置中的操作预设与自定义键位编辑器、Core 创角六维预览、390px 说明页展开明细无横向溢出、初始经验空条、菜单帮助及精简后的地图操作栏。
- 原生键盘验证从游戏脱下装备后返回游戏，再进入物品栏按 w 穿戴后保持物品栏。脚本等待实际回合更新完成再发下一操作，避免把传输尚未完成误判为快捷键失效；无运行时错误。用户全局偏好在烟测后恢复，未修改现有角色。

完整创角双语／缩放／多分支矩阵、自然战斗中的 Tab 和全部职业成长未逐项实机验收；不以本次烟测替代这些范围。

证据：`test-results/release-0.0.5-build.log`、`test-results/release-0.0.5/report.json` 及同目录截图。聚焦测试与 debug 烟测范围见[状态快照](status.md)和[创角明细计划](character-creation-details-plan.md)。
