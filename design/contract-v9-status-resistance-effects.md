# Contract v9：状态、抗性与效果管线

状态：历史 contract baseline；已由 contract-v10 接替

## 1. 目标

阶段 B 要把伤害、治疗、状态和抗性从近战、物品、怪物 blow 与未来法术中抽成同一组权威规则原语。contract-v9 已把第一批状态与抗性接入运行时、协议、存档和 state hash；contract-v8 继续作为历史行动调度基准保留。

首批玩家可见范围：

- 状态：毒、流血、眩晕、恐惧、加速、减速；
- 伤害：物理、酸、电、火、冷、毒；
- 效果：伤害、治疗、添加状态、移除状态；
- 抗性：弱点、普通、抗性、强抗、免疫；
- 时间：状态持续时间使用 `worldTick`，每个 tick 阶段的执行顺序固定并进入回放/state hash。

## 2. 已建立的纯 Rust 底座

`rfb-core` 已新增：

- `DamageType`、`DamagePacket` 和 `DamageOutcome`；
- `ResistanceLevel` 与稀疏 `ResistanceProfile`；
- `StatusInstance { kindId, intensity, remainingTicks, sourceId }`；
- `Replace`、`Extend`、`KeepStrongest` 三种显式叠加策略；
- `EffectSpec` 与结构化 `EffectOutcome`；
- 按状态 kind ID 稳定排序的持续时间推进和过期结果；
- 当前玩家/怪物物理近战通过新伤害解析边界，普通抗性下结果与 contract-v8 完全相同；
- `Actor` 权威状态保存排序后的状态列表和稀疏抗性 profile；
- 加速/减速按每级 ±10 修改派生速度，但不覆盖内容定义的基础速度；
- 毒素在每个 `worldTick` 的状态 phase 先结算抗性伤害，再衰减持续时间；
- 怪物因状态死亡时在本 tick 的能量行动前移除，玩家死亡会停止后续队列；
- 协议 1.9、state hash Schema v9 和 36 个 exact contract-v9 fixtures 已建立。

旧 RFB 的低级元素抗性关系用于校准首版等级：单层抗性约减伤 50%，更强一层约减伤 65%，弱点约增伤 50%，免疫归零。新底座暂时使用无随机抖动的整数结果；在元素伤害成为玩家可见规则前，需用 contract fixture 明确决定是否加入由权威 RNG 驱动的 RFB 风格小幅波动。不会复制旧抗性数组、随机函数或 C 分支结构。

## 3. 已完成的激活范围

1. 状态列表与抗性 profile 已接入玩家/怪物权威运行状态；
2. 独立存档 DTO 已保存状态来源和抗性，旧 v1 缺失字段按空集合迁移；
3. 协议 DTO 输出玩家状态/抗性和可见怪物状态，state hash Schema 升级到 v9；
4. 固定 tick phase 已覆盖持续毒伤、状态衰减、速度刷新和死亡中断；
5. contract-v9 在迁移的 32 个场景之外新增加速、减速、毒抗和怪物毒死 4 个 exact fixtures；
6. Fluent 已增加状态事件文本，Tauri 前端可以格式化状态伤害、过期与死亡事件，并在 HTML 状态层显示玩家当前状态、强度和剩余 tick。

下一步扩展流血、眩晕、恐惧及酸、电、火、冷的实际伤害入口。

## 4. 边界与未决决定

- 状态 kind 使用稳定字符串 ID，不使用 Rust 枚举序号进入存档；
- 真实状态与玩家知识状态分离，普通协议以后只输出玩家可知部分；
- 抗性来源最终由派生属性管线合并，运行时不能散落多个 `has_res_fire` 布尔值；
- 效果执行只返回结构化结果，不拼接本地化文本、不直接修改 PixiJS/UI；
- 同一 tick 内按实体实例 ID、效果 phase 和状态 kind ID提供完整 tie-breaker；
- 加速/减速只改变派生速度，不能直接递归执行额外命令。
