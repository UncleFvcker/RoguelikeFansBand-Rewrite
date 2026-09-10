# 文档索引

当前指南从代码与实际工作方式整理，历史设计和逐批实现记录单独归档。先读与任务有关的一页，不需要按顺序读完所有材料。

| 文档 | 回答的问题 |
| --- | --- |
| [开发与构建](development.md) | 如何准备环境、启动、生成独立程序，产物在哪里？ |
| [架构与代码地图](architecture.md) | 一条命令如何进入核心，某项行为应该改哪里？ |
| [内容开发](content-development.md) | 如何核对原版、增加内容、接入入口与更新锁？ |
| [验证与契约](testing.md) | 本次该跑哪些检查，何时更新 fixture 或扩大测试？ |
| [并行协作](parallel-development.md) | 三个工作树、四个对话如何交付与合并？ |
| [状态快照](status.md) | 哪些内容存在、哪些入口开放、哪些流程有验收记录？ |
| [后续工作](next-work.md) | 当前明确缺口是什么，下一批如何选择？ |
| [创建角色 UI 计划](character-creation-ui-plan.md) | 如何把创角长表单改为约占屏幕七成的多级选择面板？ |
| [历史档案](archive/README.md) | 去哪里查旧版设计、来源审计与过去的验收记录？ |

[AGENTS.md](../AGENTS.md)集中维护工作约定；机器可读事实在 [Cargo.toml](../Cargo.toml)、[package.json](../web/package.json)、[协议定义](../crates/rfb-protocol/src/lib.rs)、[正式内容](../packs/rfb-demo-original/)和[契约政策](../tests/fixtures/active/baseline-policy.json)。文档提供定位与解释，不复制这些文件成为第二份规格。

## 如何维护

- 入口、版本或已验收范围变化时更新状态页；命令或职责变化时更新对应指南。
- 功能提交写清行为与验证即可，不要求每批新增设计文档、审计表和交接报告。
- 有持久价值的决策说明问题、选择、依据与适用范围；过期后归档。档案内的“当前”“必须”“下一步”只代表其记录时点。
- `design/` 留下的少量短入口服务于已经启动的对话，新增文档使用 `docs/`。
