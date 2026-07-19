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
   同时验证协议 1.17 的玩家状态列表为空且 HTML 状态层显示“无”；
2. 小键盘 5 等待后回合增加、消息写入 HTML 列表、地图更新 0 格；
3. 小键盘 6 东移后位置变化，合并规则格与 Rust 权威 FOV/光照增量后更新 99 格；
4. 导出 `.rfbsave`，校验文件名、非空字节和成功消息；
5. 保存后继续移动，再从捕获的真实 Blob 载入，恢复回合和位置；
6. 导出 `.rfbreplay`，校验文件名、非空字节和成功消息；
7. 在整图与玩家居中镜头间切换，移动到可跟随区域并验证相机偏移、边缘钳制、state hash 和 dirty cell 计数不受镜头切换影响；
8. 从 100% 切换到 150% 缩放再恢复，验证相机偏移、视口尺寸、Canvas 身份、state hash 和 dirty cell 计数；
9. 从 ASCII 热切换到原创图片 tileset，重用同一 Canvas 并重绘 400 格。
10. 创建桌面原生命名存档槽，验证地点、回合和状态摘要；移动后载入并恢复 state hash；
11. 原生载入后继续派发命令，验证 TypeScript command sequence/revision 与 Rust 会话同步；
12. 覆盖并删除原生槽，同时保留手动 `.rfbsave` 导入/导出场景。
13. 验证默认 16×16 terrain chunk 初始重建、普通 dirty update 零重建、tileset 全量失效和累计重建计数；
14. 验证整图、玩家居中、跟随移动和 150% 缩放下的 4/1/2/1 个可见 chunk。
15. 从 5 个发光碎片中指定丢弃 2 个，验证背包剩余 3 个和单堆数量事件；随后拾取回声护符；
16. 单选装备回声护符，验证攻击 2→3、防御 1→2、最大生命 10→14，以及三个装备修正词条；卸下后恢复基础属性，再多选两堆物品执行整堆批量丢弃；
17. 从操作前的 `.rfbsave` 恢复背包、装备、地面物品、回合和位置，确认 UI 选择状态和数量输入不进入存档；
18. 合成 WebView `ErrorEvent`，验证前端未处理异常通过 Tauri IPC 自动生成 `.rfbdiagnostic`，并显示脱敏且不自动上传的中文提示。
19. 显式启用开发诊断钩子，运行 192×64 原创大地图 profile，对比 8/16/32 格 chunk；校验整图理论值 86,016 个动态 display object 在可见 chunk 复用后分别降到 7,168、7,168 和 28,672，并验证 active/pooled chunk 与有限值性能结果。

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
- `test-results/render-profile.json`：成功场景生成的大地图 profile Schema v1。

WebDriver 构建还会把桌面日志和崩溃诊断目录重定向到 `test-results/`，避免强制结束测试进程时在真实应用目录留下异常退出标记。

E2E 启动的 WebView2 固定追加 `--disable-gpu`，使用软件合成避免桌面会话中共享 GPU 通道偶发挂起。测试仍覆盖真实 PixiJS/WebView2/Tauri 跨层链路；`render-profile.json` 用于同一 CI 环境内比较对象数量、缓存行为和性能趋势，不作为玩家机器 GPU 性能基准。

设置 `RFB_E2E_CAPTURE_SCREENSHOT=1` 时，成功场景还会写入 `test-results/tauri-e2e-success.png`，用于人工检查 chunk 接缝、tileset、光照和遮罩。

该目录已被 Git 忽略。CI 失败时上传截图与日志；成功时单独上传 `tauri-render-profile` artifact，便于比较不同提交和 Windows runner 的趋势。

## 5. 后续扩展

- resize、DPI/缩放与最小化/恢复；
- 稳定截图基准和可控像素容差；
- 键位三预设的焦点与文本输入隔离；
- 损坏存档、无效 tileset 与核心错误的 UI 恢复；
- 通过受控测试入口模拟主文件损坏，验证 UI 的 `recoverable` 状态和备份提示；
- Android Appium 场景复用同一语义断言。
