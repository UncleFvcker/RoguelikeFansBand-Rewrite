# v0.0.4 便携版发布验证

2026-09-16。整合角色／镜头平滑移动、逐格光照与视野同步过渡、近战碰撞、远程攻击动画和法术卡片。

## 版本与产物

- Cargo workspace、npm、Tauri、EXE FileVersion／ProductVersion 均为 `0.0.4`。
- 协议 `1.300`、save payload `46`、State Hash Schema `151`、内容包及 lock `1.486.0` 保持不变。本批不新增旧开发存档迁移。
- `npm run build -- --no-bundle` 构建优化后的普通 Windows x64 standalone，产物为 `target/release/rfb-tauri.exe`，不包含 WebDriver 专用接口，不生成 NSIS。
- EXE SHA-256：`58d5ffb6282675fe08f7a53da2669071d0efa4c5202bb8d66165c8cd9b590eea`。
- ZIP 根目录仅含 `rfb-tauri.exe` 和 `LICENSES/`；不附带 userdata、个人偏好、调试符号或测试资料。内容、界面和内置 tileset 嵌入 EXE，SOURCE 指向本次发布提交和 v0.0.4 源码。
- 地图沿用原逐格光照；已撤回的地图光照 shader 不在本次发布内。标题页 shader 和 unique 彩虹保留。

## 验证与修复

- 复用刚完成的前端 303 项、核心相关 73 项、原生偏好 1 项验证。前端首轮三个旧事件样例漏写必需 kind，修正后相关 68 项复验通过。类型检查发现 SettingsRenderer 方法声明遗漏，standalone 编译发现事件投影使用的 serde_json 仅为 dev-dependency，均已修复。
- 0.0.4 正式构建重新完成 TypeScript 检查、Vite 打包和 Tauri 优化构建。Vite 保留已有大文件体积提示，不影响构建结果。
- 两条射击契约 `481-weapon-proficiency-ranged-growth`、`50-player-projectile-launcher-unavailable` 首验只缺新增的动画起止事件。独立比较确认完整 finalState（含状态哈希）、原有全部事件逐字不变，再仅刷新这两个 fixture；两项复验通过。未刷新其他基线、状态哈希或 RNG 预期。
- **普通 release EXE** 在独立安装／WebView 目录正常新建人类心灵术士，确认逐格光照、实际单步插值、700／1280／1920 宽度的 2／3／4 列法术卡片、伤害文本及大写字母展开，无溢出或运行时异常。移动采样 53 帧。没有改动用户现有角色；没有运行全量测试。
- 本轮桌面烟测未逐项实机施放所有近战、箭矢、beam、ball、storm 和陨石群，不将算法测试或构建通过等同于这些动画的全面视觉验收。长程帧率、全部职业自然成长和 Android 不在本次范围内。

证据：`test-results/release-0.0.4-build.log`、`release-0.0.4-smoke.log`、两份射击 observe JSON，以及 `test-results/map-motion-spells/report.json` 和截图。自动测试证据为 `test-results/animation-*.log`。实现及用例范围见[角色移动](player-movement-plan.md)和[远程动画](projectile-animation-plan.md)。
