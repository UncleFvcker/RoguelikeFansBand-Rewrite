# Contract v16：核心目标选择与非八方向轨迹

状态：协议 1.16 / contract-v16 active baseline

## 已建立边界

- 核心通过 `TargetSpecDto` 声明方向、格子和实体三种选择模式、最大射程及视线要求；当前发射器 profile 直接携带该规格。
- 新命令 `FireTarget { target }` 接受稳定 `TargetSelection`。实体目标使用实例 ID，并在命令执行时解析其当前位置；协议 1.14 的 `Fire { direction }` 继续兼容旧回放。
- 格子和实体目标必须位于地图内、不是玩家自身、处于射程和当前可见范围内。无效目标在弹药消费和 RNG 之前产生确定性 unavailable 事件。
- 非八方向目标使用整数 Bresenham 路径；方向目标保持原有逐格轨迹。两者继续在阻挡地形或首个实体处停止，并输出相同的结构化 trace。

## 协议与兼容性

- 协议升级至 1.16；内容包继续为 1.11.0，content hash 不变。
- save schema v1 / state hash schema v9 不变；目标选择与轨迹都是命令期间的瞬时输入和输出。
- contract-v16 从 v15 迁移 52 个 exact fixtures，并新增一条非八方向实体目标 fixture，共 53 个。

## 后续

下一切片把核心声明的规格接入前端目标模式和输入命令；弹药破损/回收、投掷命中与伤害仍在后续规则切片实现。
