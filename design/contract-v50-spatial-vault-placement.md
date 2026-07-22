# Contract v50：Vault 空间变换与确定性多模板落位

## 目标

contract-v50 在 v49 的 actor/loot 总预算之上补齐第一版 Vault 空间管线：模板可声明旋转和镜像，同层可放置多个 Vault，落位受数量、面积、actor 和 loot 四类预算共同约束，无法落位的候选以稳定顺序回退。

## 内容契约

- `VaultDefinition.transforms` 是可选、去重的变换列表，支持 identity、四向旋转和四种镜像；空列表保留 v49 identity 行为。
- Vault 尺寸范围扩为 2–12，入口可位于任一边界格，不再固定为顶边中心。
- `generationBudget.vaultPlacements` 与 `vaultAreaTiles` 必须成对出现；范围分别为 1–4 和 4–512，且面积不能超过楼层内框。
- 空间 Vault 预算只允许普通 dungeon 楼层使用，并要求 theme/encounter/loot 表；每个深度可用主题必须至少提供 `vaultPlacements` 个不同候选。
- 编译器继续保证每个候选单独预留固定群体和 loot 后，至少剩余一个普通 encounter 与一个普通楼层 loot placement。

## 确定性生成顺序

1. 按既有规则选择深度主题并收集深度可用 Vault 候选。
2. 从 actor/loot 总预算中先保留守护者、巢穴以及一个普通 encounter/loot placement；剩余预算可供 Vault 竞争。
3. 每个落位槽只考虑尚未尝试且同时满足剩余面积、actor 和 loot 预算的候选，并按内容权重抽取。
4. 模板变换按枚举规范序、地图原点按行优先枚举。候选矩形必须完全位于边框内的未开凿 wall 区，变换后的入口外侧必须连接已有非 wall 地形。
5. 可行落位超过一个时只进行一次有界抽取；绘制后，该矩形因不再是 wall 而自然拒绝后续重叠。
6. 若所选模板没有可行原点，候选被稳定移除，但不消耗落位槽或空间/actor/loot 预算；随后继续候选回退。候选耗尽时结束，不强行生成。

同一 Vault 在一次楼层生成中最多选择一次。空间 Vault actor 实例 ID 含稳定 placement ordinal；terrain、actor、item 与 RNG 仍直接进入既有存档和 state hash。

## 原创场景与验证

- 共鸣压力地牢深度 8 声明 2 个 Vault、180 格面积、9 actor 和 3 loot placement 预算。
- 两个小型 Vault 分别覆盖八向变换与非对称镜像，最终占用 2 actor/2 loot，普通生成保留 7 actor/1 loot。
- 12×12 封闭巨构以高权重进入候选，但在双房间地图上没有合法 wall 矩形；生成器稳定回退到两个小模板。
- `contract-v50` 从 v49 迁移 99 个 exact fixtures，并新增深度 8 空间 Vault fixture。active baseline 共 100 个 exact fixtures，0 waiver。
- 单元测试覆盖变换坐标、重复变换/内部入口拒绝、空间预算成对校验、重叠排斥、失败回退和 v49 已生成楼层迁移。

## 版本

- 协议：1.50；
- 内容包：1.43.0；
- save：v1，不增加字段；
- state hash：Schema v19，不增加字段；
- active baseline：contract-v50，100 exact fixtures。

## 明确延后

- 多入口、入口方向权重、大模板成功落位后的全图连通性证明和跨走廊拼接；
- 房间数量/形状、陷阱、门与特殊地形预算，以及同层多区域主题；
- friends、escort、pit、formation、pack AI、召唤、繁殖、种群上限和 unique 过滤；
- 分支、shaft、随机楼梯、同层多个连接点与独立到达点。
