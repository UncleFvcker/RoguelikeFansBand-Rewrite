# Fluent 本地化运行时 v1

状态：首个桌面双语切片已实现

## 1. 边界

- Rust 核心只输出稳定事件 key、语义 ID 和参数，不输出本地化完成的业务句子。
- TypeScript 桌面 UI 使用 `@fluent/bundle` 格式化界面、消息、背包名称和按键提示。
- `rfb-localization` 使用 Rust `fluent-bundle` 加载同一套资源，供未来 CLI、原生诊断和非 WebView 消费者复用。
- 当前语言保存在 `localStorage` 的 `rfb.locale`，不进入存档、回放、命令、RNG 或 state hash。
- 默认语言为 `zh-CN`，`en-US` 作为完整的兼容资源和技术回退；自动检查要求简体中文不得缺少已发布 key，因此正常发行流程不应因回退而向玩家暴露英文。英文也缺失时开发界面显示 `[message-key]`。

## 2. 资源结构

```text
locales/
├─ en-US/
│  ├─ ui.ftl
│  ├─ game.ftl
│  └─ content.ftl
├─ zh-CN/
│  ├─ ui.ftl
│  ├─ game.ftl
│  └─ content.ftl
└─ glossary/
   ├─ style-guide.zh-CN.md
   └─ terms.zh-CN.yml
```

这些资源均为新项目原创文本，不读取或复制旧 RFB 的 `localization/`、旧 TSV 或旧译文。

## 3. Key 规则

- 使用小写 kebab-case，例如 `message-combat-hit`、`ui` 资源中的 `inventory-stack-count`。
- 前缀表达消费者或语义：`action-`、`connection-`、`controls-`、`message-`、`item-`、`actor-`。
- key 不使用英文原句、数组序号或屏幕位置。
- 参数必须使用有意义的名称，例如 `$target`、`$damage`、`$quantity`。
- 英中资源必须具有相同 key 集合和参数集合，但参数顺序与完整句式可以不同。

## 4. 消息历史与语言切换

消息面板保存结构化 `GameEventDto` 或本地系统消息 key/参数，不保存已经格式化的字符串。语言切换时重新格式化整个消息历史、背包和按键提示，因此已有消息也会切换语言。

Rust 规则层不直接创建 message key 或字符串参数。物品、移动和战斗先产生带类型字段的 `DomainEvent`，命令结算完成后再按原始事件顺序投影为 `GameEventDto`；数字格式化和 Fluent key 映射只发生在该投影边界。后续任务、经验、知识发现和统计系统应订阅领域事件，不解析本地化 DTO。

内容名称通过稳定 kind ID 映射到 Fluent key。逻辑层不会根据“发光碎片”或 `luminous shard` 判断物品类型。

## 5. 自动检查

`npm test` 当前检查：

- 英中 key 集合完全一致；
- 同一 key 的具名参数集合一致；
- 两种语言均可被 Fluent 解析；
- 英文复数和中文独立语序能够正确格式化；
- 当前语言缺少 key 时回退英文；
- 前端源码不重新加入高置信中文硬编码或直接 DOM 文本字面量。

Rust workspace 测试同时加载并格式化两种语言资源。桌面 E2E 验证语言切换不会改变 state hash、回合或地图 Canvas，并会重新渲染历史拾取消息。

## 6. 后续

- 把 Tauri/tileset 技术错误改为稳定错误代码和本地化错误 key；
- 从内容视觉目录进一步输出 `nameKey`，移除前端临时 kind ID → key 映射；
- 增加伪语言、超长文本和窄窗口布局测试；
- 建立 glossary-check、unused key 和更完整的英文硬编码扫描；
- 迁移装备、丢弃、商店、设置和文件选择流程。
