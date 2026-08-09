# Contract v218：十六级怪物 P33 阻塞收口

## 范围

本批从权威 RFB `master` Git 对象接入恐爪怪（283）、夸塞魔（294）、老鼠王子尼祖基尔（1299）和布伦比野马（1334）。中文名严格使用权威中文表。

## 窄机制

- `HURT_ROCK`：恐爪怪记录为既有 `disintegrate` 易伤；未来黏土魔像可复用同一映射，不新增碎岩伤害框架。
- `CAN_CLIMB`：actor movement 增加 `climb`，只让匹配 actor 进入带同一 mode 的山地与冰川。
- `TELE_LEVEL`：新增怪物专用 `teleport-level` ability effect。命中玩家后先按 55 点 Nexus 抗性池判定，再走既有 saving throw；成功时复用现有楼层目标、切换、存储和事件事务。它不能用同层传送近似。
- `COMPOST`：actor allocation 增加可选 `taskId`，映射为 `demo.task.the-sewer`。全局分配和分类召唤都会过滤不匹配任务；本批不创建下水道地图、设施、奖励或兼容层。
- `FIXED_UNIQUE` 与 `NO_QUEST`：保留为源事实 tags。现有运行时没有 Unique 抑制或通用随机任务目标选择器，因此不增加无消费者状态。
- `SMART`：导入到既有 monster casting `smart`，同时补正已选择施法怪物的同一源事实。

## 内容与版本

- 严格选择记录：284。
- 正式包：`1.214.0`，88 terrain、349 actors、249 items、147 abilities。
- Content hash：`94b3ab337a895fc36e1240bd2107c4f39ba259b6c814080333ddaa441119b451`。
- 协议：`1.152`；新增 `teleport-level` spec 与 resolution。
- State Hash Schema：`v72`，不变；没有新增权威状态结构。
- 行为基线：`contract-v218`，470 exact fixtures，零豁免。

## 验收

聚焦契约覆盖四只 actor、解离易伤、山地/冰川攀爬、下水道任务分配过滤、`heal-48` 和跨层传送的抗性/豁免/楼层事务。协议与内容结构变化后重新生成 schema、TypeScript bindings，并刷新完整 active baseline。
