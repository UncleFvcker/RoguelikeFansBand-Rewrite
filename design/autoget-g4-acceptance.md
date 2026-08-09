# G4：协议与验收收口

状态：已完成。最终协议 `1.156`，state hash Schema `v76`，contract 基线 `v219`；save 容器保持 v1，内容包版本和内容哈希不变。

## 版本说明

最初计划的协议 `1.154` / Schema `v75` 已由 G0 的 `autoGetMode` 使用。G1 新增金币发现权威状态后升至协议 `1.155` / Schema `v76`；G2 的 `AutoGet` 命令使协议升至 `1.156`。G3–G4 未新增协议字段或 state-hash 输入，因此不再制造空版本。

## 验收范围

- Web：Ctrl+G 与小写 `g`、世界地图禁用、锁定目标与重新选取、全部中断条件。
- Core：`off / ammo / wanted`、金币、`=g`、拾取与销毁规则、受保护物品、最近目标、稳定 ID、投射例外和绕路。
- 持久化：每角色模式与金币发现状态的 save、回放和 state hash。
- 契约：统一刷新并验证全部 470 条 active exact fixtures，零 waiver。

按照阶段边界，不执行独立桌面构建。
