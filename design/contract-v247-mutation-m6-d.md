# Contract v247: mutation M6-D

状态：active baseline。协议保持 `1.166`，save 容器保持 v1，State Hash Schema
保持 v83。内容包为 `1.239.0`，content hash 为
`cad1af06638d417216dd7defc46d5fdaf3922720cf719bb31a7dd45ea8e49560`。

本批激活七项 RFB 周期变异：Normality、Wraithform、Polymorph Wounds、Wasting、
Random Telepathy、Nausea 与 Warning。它们按 `sourceIndex` 进入既有唯一周期入口，
世界地图继续零触发、零额外 RNG。

- Normality 通过统一随机失去事务移除一个未锁定变异。
- Polymorph Wounds 与主动 Polymorph 共享同一伤口变形事务。
- Wasting 复用属性衰减并尊重六项 Sustain，保持原版抽取顺序。
- Random Telepathy 和 Wraithform 复用可持久化的通用状态；没有新增状态字段。
- Nausea 复用营养状态，Warning 按当前层全部存活怪物的等级计算危险度。

变异账本现为 114 active / 38 blocked；28 项周期变异中 27 项 active，104 个随机
候选中 99 项 active。行为基线推进到 `contract-v247`，现有 21 个 exact fixture
无需刷新；七项分支、锁定保护和 RNG 顺序由 Core 聚焦测试覆盖。
