# 桌面崩溃诊断闭环 v1

状态：Windows/Tauri 自动本地诊断已实现

## 1. 目标与边界

桌面版不要求玩家在崩溃后主动点击“导出”。应用在私有日志目录维护活动会话标记，并自动生成版本化 `.rfbdiagnostic` 报告。报告只保存在本机，不自动上传，也不在崩溃进程中弹出文件选择器。

当前覆盖：

- Rust panic：panic hook 记录脱敏源码文件名、行号和列号；下一次启动生成报告；
- 未正常退出：活动会话标记未被正常清理时，下一次启动生成报告；
- TypeScript 未处理的 `error` / `unhandledrejection`：运行中的 Tauri 进程立即生成报告；
- 正常退出：Tauri `RunEvent::Exit` 删除活动会话标记，不生成报告。

系统强杀、访问违规、断电或操作系统崩溃不保证能够在当时执行 panic hook，但下次启动仍可根据未清理标记识别异常退出。v1 不生成 Windows minidump。

## 2. 存储与轮换

- 日志仍使用 Tauri `app_log_dir()/rfb-desktop.log`；
- 活动标记和报告位于 `app_log_dir()/diagnostics/`；
- 活动标记固定为 `active-session.json`；
- 报告命名为 `crash-<Unix 毫秒>[-序号].rfbdiagnostic`；
- 最多保留最近 5 份报告，生成新报告后自动删除更旧文件；
- WebDriver E2E 通过仅在 `webdriver` feature 生效的环境变量重定向到仓库内被忽略的 `test-results/`，不污染玩家数据目录。

前端只接收报告文件名、生成状态和异常分类，不接收诊断目录的绝对路径。

## 3. 报告格式 v1

`.rfbdiagnostic` 当前是便于审查的 UTF-8 JSON：

```json
{
  "format": "rfb-diagnostic",
  "formatVersion": 1,
  "generatedAtUnixMs": 0,
  "reason": "unclean-exit",
  "appVersion": "0.1.0",
  "protocolVersion": "1.31",
  "operatingSystem": "windows",
  "architecture": "x86_64",
  "contentId": "rfb.demo.original-v1",
  "contentHash": "...",
  "rendererBackend": "pixi-layered-chunks-v3",
  "previousSessionStartedAtUnixMs": 0,
  "panicLocation": null,
  "logTail": []
}
```

格式独立于核心协议、存档和 state hash；以后扩展字段时必须增加默认值或提升 `formatVersion`。

## 4. 隐私与大小限制

报告默认不包含：

- 完整存档或背包/角色数据；
- 玩家输入文本和存档显示名称；
- 用户名、网络凭据或任意绝对路径；
- WebView 浏览历史、缓存或系统环境变量；
- 自动上传地址或网络请求。

报告最多读取日志末尾 256 KiB，并把文本日志重新解析成结构化白名单事件。未知事件保留事件名但丢弃 detail；panic 路径只保留源文件 basename、行号和列号。前端异常 v1 只记录 `window-error` 或 `unhandled-rejection` 分类，不保存可能包含玩家数据的异常 message/stack。

## 5. 玩家反馈

自动生成报告后，消息面板通过 Fluent 显示报告文件名，并明确提示报告不会自动上传。没有手动日志导出按钮，也不会因为诊断读写失败把正常核心会话改为规则错误状态。

## 6. 验证

Rust 单元测试覆盖：

- 未正常退出标记在下一次启动转换为报告；
- 前端未处理异常立即生成报告；
- 未知日志 detail 和绝对 panic 路径被清除；
- 正常退出清理标记；
- 报告轮换上限。

Windows Tauri E2E 使用合成 `ErrorEvent` 验证真实 WebView → Tauri IPC → Rust 文件写入 → Fluent 消息闭环，并确认文件名、原因和 `.rfbdiagnostic` 后缀。

## 7. 后续

- 根据真实硬崩溃案例决定是否接入 Windows minidump；
- 评估在不包含完整存档的前提下加入最近回放检查点和 state hash；
- Linux/macOS 桌面运行目标建立后复用相同目录与生命周期测试；
- Android 真机测试仍按总规划暂缓。
