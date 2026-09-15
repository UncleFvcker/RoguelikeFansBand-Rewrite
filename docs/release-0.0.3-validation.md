# v0.0.3 便携版发布验证

2026-09-16。整合紧凑与可隐藏的 HUD、设置菜单、法术字母选择、六组 60 格快捷栏、RFB 法力周期与小数状态，以及连续休息中断修正。

## 版本与产物

- Cargo workspace、npm、Tauri、EXE FileVersion／ProductVersion 均为 `0.0.3`。
- 协议 `1.300`、save payload `46`、State Hash Schema `151`；内容包及 lock `1.486.0`。生成绑定、协议 Schema、内容 lock 已检查。
- `npm run tauri -- build --no-bundle` 构建优化后的普通 Windows x64 standalone，产物为 `target/release/rfb-tauri.exe`，不含 WebDriver 专用接口，不生成 NSIS。
- EXE SHA-256：`9e6965adc3310d63bf0916aa4533f0a6667501b0285c6262ec2435147896438d`。
- ZIP 根目录仅含 `rfb-tauri.exe` 和 `LICENSES/`，不含存档、偏好、测试文件或开发依赖。内容、界面与内置 tileset 嵌入 EXE；许可证目录保留来源与第三方声明，SOURCE 指向本次发布提交。
- 角色存档格式有变化，不兼容 0.0.2 开发存档；保留旧版目录与 userdata，在新版目录中新建角色。

## 验证结果

- 复用本批已通过的 446 项核心相关用例、254 项前端用例、桌面偏好 8、协议 7、保存 2、逐回合休息保存回放 1 项；类型、相关 Rust 格式和核心／桌面库 Clippy 通过。没有重复全量测试。
- `contract-v347` 的 26 条共享契约通过。相对前版只改变小数状态及哈希版本相关的状态哈希，事件、行动结果、时间与 RNG 断言不变。发布检查同步库内 `ACTIVE_BASELINE` 到 policy 后，2 项基线与共享常量测试通过。
- 浏览器真实按键 R → Enter 在模拟核心响应下连续三次执行，Esc 取消不发命令；中英文宽窄屏、法术操作、三栏折叠与菜单关闭检查通过。修正菜单展开新增边框引起的 1 像素布局跳动；同步旧界面测试准备中的物品外观、任务放弃许可和未达等级能力状态。
- **普通 release EXE** 在独立安装与 WebView 目录中新建人类心灵术士，以 WebView2 CDP 发送实际键盘事件：R 打开默认 `&` 输入框，输入 3 并 Enter 后由真实 Rust 核心连续处理三次休息，回合从 0 到 3，没有前端取消消息；再次 R／Esc 没有新命令。六组切换入口、十个当前快捷格、法力 HUD 和原生保存通过，未捕获运行时异常。
- 冒烟仅读取既有全局偏好，未修改现有便携目录或角色状态。调试连接首轮在 WebView 页面尚未创建时过早连接，改为等待目标页面后通过；没有因此修改生产代码。

证据在忽略目录 `test-results/release-0.0.3/`，含 `smoke.mjs`、`smoke-report.json`、`smoke.rfbsave` 和 `game.png`；此前细项见[法力审计与验证](mana-system-rfb-audit.md)。这些检查不代替长期游玩、所有职业自然成长或 Android 验收。
