# Tauri 桌面端到端测试

状态：Windows WebView2 玩家流程 E2E 已接入按路径触发的 CI；渲染架构实验单独运行。

## 1. 目标与边界

桌面 E2E 验证真实 Tauri 窗口、WebView2、Tauri Commands、Rust 会话和 PixiJS/HTML 前端之间的完整链路。它不替代 Rust 单元测试、contract fixtures 或 Node manifest 测试，只覆盖跨层集成风险。

测试不读取旧 RFB 源码、文本、存档或素材，也不把旧内容复制到仓库或测试产物。

## 2. 驱动架构

`tauri-plugin-wdio-webdriver` 作为可选 Cargo 依赖，仅在 `webdriver` feature 下编译。测试使用 Node 内置 `fetch` 直接调用嵌入应用的 W3C WebDriver HTTP 接口，不引入 WebdriverIO、EdgeDriver 或浏览器自动下载。

安全边界：

- `webdriver` feature 在 release profile 下触发编译错误；
- debug 程序只有存在 `TAURI_WEBDRIVER_PORT` 时才注册驱动；
- 普通开发、正式构建和发行包均不包含可用的自动化入口；
- CI 和本地测试使用随机回环端口。

## 3. 当前场景

当前主场景从标题页创建战士（种子 `42`）和死亡领域高阶法师（种子 `7`），验证：

1. 新游戏进入可玩状态，Canvas 存在，协议和内容哈希与构建源一致；
2. 背包、人物、任务、能力、设置菜单可打开关闭，背包快捷键可切换，菜单不推进状态；
3. 镜头、缩放、ASCII/图片外观可切换，不改变 state hash，并复用 Canvas；
4. 从当前装备中选择一件，卸下后进入背包，再穿回原槽；
5. 战士和法师均导出真实 `.rfbsave` Blob，继续行动后导入，精确恢复哈希、回合、位置、装备及资源，再继续行动；
6. 创建原生命名存档，重载至标题页后载入并恢复哈希，清理测试槽；
7. 法师学习第一项可学法术并成功施放，资源变化且能力菜单关闭；
8. 导出非空 `.rfbreplay`，合成 WebView `ErrorEvent` 验证自动生成 `.rfbdiagnostic`。

通过后写入 `test-results/playable-acceptance.json`。场景已跟随当前初始世界和本地化回合文本更新，不再绑定已移除的 20×20 演示场景、初始地面碎片或旧引导面板。旧场景中的固定移动轨迹、部分数量丢弃与原生覆盖交互不属于当前版本主场景覆盖范围。

玩家流程保留镜头、缩放、tileset 热切换和 Canvas 复用检查，不断言渲染后端名称、图层排列、显示对象数量或 chunk 数量。

`MapRenderer` 在 `#map-host` 暴露只读诊断属性：最近渲染类型、最近处理格数、累计处理格数、当前 tileset ID、镜头模式、缩放、相机偏移、视口尺寸、visible/remembered/hidden 格数量，以及 terrain chunk 总数、可见数、剔除数和重建计数。这些信息不影响游戏规则、存档或状态哈希。

## 4. 本地运行与诊断

```powershell
cd web
npm ci
npm run e2e
```

`e2e:build` 使用 Tauri debug/no-bundle 构建并嵌入 Vite 产物，`e2e:tauri` 启动程序和随机端口驱动。失败时自动写入：

- `test-results/tauri-e2e.png`：当前窗口截图；
- `test-results/tauri-e2e.log`：应用 stdout、stderr 和退出状态。
大地图 profile 使用独立命令，不包含在 `npm run e2e` 中：

```powershell
npm run e2e:build
npm run e2e:render-profile
```

该命令复用 E2E 驱动与调试程序，在标题页独立运行 `render-profile.e2e.mjs`，不执行玩家流程。它启用诊断钩子，对比 192×64 大地图下 8/16/32 格 chunk 的对象数量、复用行为和性能，保留实验的架构基线断言，成功后写入 `test-results/render-profile.json`。

WebDriver 构建还会把桌面日志和崩溃诊断目录重定向到 `test-results/`，避免强制结束测试进程时在真实应用目录留下异常退出标记。

E2E 启动的 WebView2 固定追加 `--disable-gpu`，使用软件合成避免桌面会话中共享 GPU 通道偶发挂起。测试仍覆盖真实 PixiJS/WebView2/Tauri 跨层链路；`render-profile.json` 用于同一 CI 环境内比较对象数量、缓存行为和性能趋势，不作为玩家机器 GPU 性能基准。

设置 `RFB_E2E_CAPTURE_SCREENSHOT=1` 时，成功场景还会写入 `test-results/tauri-e2e-success.png`，用于人工检查 chunk 接缝、tileset、光照和遮罩。

该目录已被 Git 忽略。CI 失败时上传截图与日志；只有显式选择渲染实验时才上传 `tauri-render-profile` artifact。

### CI 执行范围

- `ci.yml`：每次 PR/main push 运行 Rust 检查、前端单元测试和 UI 构建。
- `windows.yml`：前端运行代码、资源、桌面壳、协议/存档/回放接口、构建配置或桌面 E2E 改动时，构建一次 E2E 调试包并执行玩家流程与补给循环；纯文档、独立测试文件和内容包改动不触发。
- `android.yml`：共享前端、原生壳、协议/存档/回放接口或构建配置改动时执行 Android ARM64 构建；桌面 E2E 脚本不触发 Android，Android 原生工程改动不触发 Windows。
- 核心规则、内容包及其他未命中路径的改动由常规检查覆盖；需要跨层验收时，或到达里程碑时，手动运行相应平台工作流。
- 手动运行 Windows 工作流还会执行独立发布构建；勾选 `render_profile` 才运行渲染架构实验。Android 工作流也支持手动运行。

## 5. 后续扩展

- resize、DPI/缩放与最小化/恢复；
- 稳定截图基准和可控像素容差；
- 键位三预设的焦点与文本输入隔离；
- 损坏存档、无效 tileset 与核心错误的 UI 恢复；
- 通过受控测试入口模拟主文件损坏，验证 UI 的 `recoverable` 状态和备份提示；
- Android Appium 场景复用同一语义断言。
