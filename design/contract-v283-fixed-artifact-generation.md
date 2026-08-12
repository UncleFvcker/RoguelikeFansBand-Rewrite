# Contract v283：固定神器统一生成底座

状态：已实现

## 边界

- 核心内部增加 `Ordinary / Good / Great / Artifact` 生成意图，不扩充协议
  `ItemQualityDto`。
- 所有生成结果先成为未分配实例 ID 的 draft；只有调用方接受后才分配 ID、创建物品并
  提交固定神器唯一状态。
- Artifact 请求先执行原版 1/10 瞬时神器入口，再选择普通基础物品；普通固定神器按
  `sourceIndex` 顺序执行已生成排除、基础物品匹配、神器 OOD 与稀有度判定，最多四次。
  瞬时神器另执行基础物品 OOD；当前内容没有瞬时神器候选。
- 固定神器实例保持普通品质且无自然 affix；Artifact 失败结果按 Great 生成，至少为
  Exceptional。
- Old Castle 的显式神器奖励保持强制授予，不因唯一集合已有记录而取消；授予后登记，
  只阻止后续随机重复。

## 持久化

`SavePayloadV1.generatedArtifactIds` 是必填的稳定 ID 数组。载入拒绝重复、未知、没有
`artifactGeneration` 的 ID，以及存在固定神器实例却没有登记的状态；已销毁神器可以只
保留登记。该集合进入 State Hash Schema v93。

## 版本与验证

- Protocol 1.187
- State Hash Schema v93
- save 容器 v1
- pack 1.294.0（内容不变）
- active baseline contract-v283，26 条 exact fixture，零 waiver

聚焦测试覆盖来源顺序、OOD、稀有度、唯一排除、Good/Great/Artifact 区分、draft 实例
序号、存档/状态哈希往返、非法存档，以及 Old Castle 奖励登记。公共存档与哈希结构改变，
因此全部 active fixture 统一刷新并复验。
