# Tauri 桌面端到端测试

状态：Windows WebView2 E2E v1 已实现并接入 CI

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

固定种子 `42` 的单一场景依次验证：

1. 初始快照绘制 400 格，地图 Canvas 与 HTML 状态/消息分别存在；
2. 小键盘 5 等待后回合增加、消息写入 HTML 列表、地图更新 0 格；
3. 小键盘 6 东移后位置变化，地图只更新旧/新位置 2 格；
4. 导出 `.rfbsave`，校验文件名、非空字节和成功消息；
5. 保存后继续移动，再从捕获的真实 Blob 载入，恢复回合和位置；
6. 导出 `.rfbreplay`，校验文件名、非空字节和成功消息；
7. 在整图与玩家居中镜头间切换，移动到可跟随区域并验证相机偏移、边缘钳制、state hash 和 dirty cell 计数不受镜头切换影响；
8. 从 ASCII 热切换到原创图片 tileset，重用同一 Canvas 并重绘 400 格。

`MapRenderer` 在 `#map-host` 暴露只读诊断属性：最近渲染类型、最近处理格数、累计处理格数、当前 tileset ID、镜头模式、相机偏移和视口尺寸。这些信息不影响游戏规则、存档或状态哈希。

## 4. 本地运行与诊断

```powershell
cd web
npm ci
npm run e2e
```

`e2e:build` 使用 Tauri debug/no-bundle 构建并嵌入 Vite 产物，`e2e:tauri` 启动程序和随机端口驱动。失败时自动写入：

- `test-results/tauri-e2e.png`：当前窗口截图；
- `test-results/tauri-e2e.log`：应用 stdout、stderr 和退出状态。

该目录已被 Git 忽略，CI 仅在失败时上传。

## 5. 后续扩展

- resize、DPI/缩放与最小化/恢复；
- 稳定截图基准和可控像素容差；
- 键位三预设的焦点与文本输入隔离；
- 损坏存档、无效 tileset 与核心错误的 UI 恢复；
- Android Appium 场景复用同一语义断言。
