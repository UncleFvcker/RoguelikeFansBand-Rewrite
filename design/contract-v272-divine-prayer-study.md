# Contract v272：神授随机祈祷学习

状态：已实现。

## 权威来源

- `D:/codex/Frogcomposband/master` 的 `master` ref，核对提交
  `efd63661302866038f58d8cd2553b23e6af3bf9d`。
- `src/cmd5.c::do_cmd_study`：失明、无光和混乱阻止学习；书本可来自背包或脚下；
  牧师式施法者不能点选祈祷，而是按书内当前可学法术顺序以 `one_in_(k)` 蓄水池抽样
  接受一次神授。

## 内容与核心

- `CastingProfileDefinition.studyMode` 只有 `chosen` 和 `divine-random`；默认值为
  `chosen`，现有高阶法师与测试施法者保持逐法术点选。
- `StudyPrayer { bookItemId }` 只接受当前领域内、位于背包或玩家脚下的能力书。
  核心按书中稳定能力顺序过滤已学和等级不足的祈祷，并使用唯一 gameplay RNG 执行
  与原版等价的蓄水池抽样。
- 学习容量、已学能力和 RNG 均复用既有权威状态；没有新增持久字段。

## 投影与界面

- `AbilityLearningDto.studyMode` 告知前端当前学习模式。
- `chosen` 继续在单个法术行显示“研习”；`divine-random` 在法术书标题只显示一个
  “学习祈祷”按钮，成功后继续复用 `ability.studied` 事件显示实际获授能力。
- 能力投影会识别玩家脚下的书本；失明、无光、混乱或没有学习容量时按钮不可用，
  核心仍重复校验全部边界。

## 版本与验收

- Protocol 1.177；State Hash Schema v88、save v1、内容包 1.272.0 与 content hash
  `2f88338bb3fe9bfa13ac703d0b58ae4521bade19619805c5fe37da977a8b4858` 不变。
- active baseline 升至 contract-v272。统一重放并复验 21 条 exact fixture，标准化结果
  零语义漂移；状态哈希本身不变。
- 定向测试覆盖模式隔离、命令分派、确定性随机授予、地面书，以及失明、无光和混乱
  三个学习阻断条件；前端测试覆盖每本书只投影一个神授入口。
