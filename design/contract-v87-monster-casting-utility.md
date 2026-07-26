# Contract v87：怪物施法效用与目标扩展

状态：当前 active baseline。协议版本为 1.87，demo 内容包版本为 1.79.0，content hash 为 `f9e9ccc93635da7f568a2cdd83f90024f86cd13d1d0ff43627f725dde4e3ecac`。save 容器继续使用 v1；本纵切没有新增权威字段，state hash 保持 Schema v37。

## 1. 原版参考与选择顺序

P27 延续 FrogComposband `monspell.c` 的分层形式：怪物行动先做施法频率检定，再由无 RNG 的 AI 过滤/调整每个法术的概率，最后从剩余概率中随机选择。它不会改成“总是选择最高评分法术”。

每个未冷却、已警戒 caster 的行动顺序固定为：

1. 抽一次 1–100 频率骰；
2. 按内容声明顺序计算全部候选的目标、footprint、拒绝原因与有效权重；
3. 频率通过且总有效权重大于零时抽一次加权选择骰；
4. 解析所选法术的效果 RNG；
5. 成功施法后设置 `ceil(100 / frequencyPercent)` 次自身行动冷却。

频率失败、全部候选被拒绝和冷却中的 RNG 边界继续沿用 v86。

## 2. 纯效用调整

内容权重仍是选择概率的基础。核心只根据当前权威状态计算有效权重：

- 自疗在损失 HP 不超过 20% 时以 `no-utility` 剔除；超过阈值后按 `ceil(missingPercent / 25)`、上限 4 倍提高权重；
- 对已有同强度或更强状态的添加、对不存在状态的移除，以及会被免疫的状态添加不提供效用；有序 sequence 只要仍有一个有效效果即可保留；
- 玩家距离施法者至少 3 格时，所有以玩家为主目标的候选权重乘 2，使远距离攻击相对于自身增益更常见；
- 计算过程不抽 RNG、不写状态，也不改变内容声明顺序。

`MonsterAbilityCandidateResolutionDto` 为每个声明候选返回基础/有效权重、目标、footprint 和 `invalid-target`、`out-of-range`、`blocked`、`friendly-risk`、`no-space`、`no-utility` 之一。选择骰只使用有效权重。

## 3. 新目标与效果

怪物执行器现在复用既有能力定义支持：

- 自身治疗、状态添加/移除与自身有序 sequence；
- 以玩家为主目标的范围爆发、延长射线和固定八向锥形；
- 以自身为中心的限时敌对召唤；
- v86 已有的直接伤害、状态和直接有序 sequence。

范围爆发使用既有 RFB 距离/衰减和中心 line-of-effect；射线沿目标继续到最大射程或阻挡；锥形使用既有八向中心线、宽度和横向衰减。`monster.ability-cast` 可返回 `affectedPositions`，召唤时还返回完整 summon resolution；自身法术和召唤没有伪造 projectile trace。

## 4. 首版风险与召唤边界

P27 对多格法术采用保守风险规则：footprint 中只要存在施法者以外的任何实体，候选就以 `friendly-risk` 拒绝。因此不会误伤同阵营怪物，也暂不把玩家召唤物当作可攻击的次级目标。后续目标 AI 可以在明确阵营/价值评分后放宽这一规则。

怪物召唤物：

- `ownerId` 是实际怪物施法者实例，而不是玩家；
- 投影为 `hostile`，可以执行普通追踪/近战/自身施法；
- 使用既有稳定 summon ID、持续回合、到期事件、当前层/离层 save 和 state hash 投影；
- 施法者死亡不提前删除已经生成的限时召唤物。

玩家拥有的召唤物仍投影为 `player`，首版友方命令/战斗 AI 保持后置。

## 5. 内容、协议与验证

demo 的 Echo Cantor 候选池加入 Mending Echo、Echo Quickening、Echo Burst、Echo Lance、Echo Fan 和 Call Discord；新增 Discordant Echo 作为敌对召唤 actor。内容包升级为 1.79.0。

协议 1.87 新增：

- `MonsterAbilityRejectionReasonDto`；
- `MonsterAbilityCandidateResolutionDto`；
- `MonsterAbilityDecisionResolutionDto.candidates`；
- `MonsterAbilityCastResolutionDto.affectedPositions/summon`；
- 无 projectile 的怪物自身法术/召唤事件。

contract-v87 从 v86 迁移全部历史场景，并新增 8 个 exact fixtures，覆盖健康自疗剔除、重伤权重与治疗、重复状态剔除、范围/射线/锥形、敌对召唤、无空间回退和存档往返。active baseline 共 257 个 exact fixtures、零 waiver；核心测试额外固定纯评估零 RNG、距离倍率、次级实体风险、敌对阵营/行动和 replay 一致性。

## 6. 明确后置

P28 候选为怪物目标选择与施法记忆：让怪物把玩家阵营召唤物作为合法目标，按敌我数量决定多目标风险，加入低 HP 逃跑/保持距离，并为 smart caster 建立基于已观察抗性的有限知识。多资源职业继续等待这一 AI 目标边界稳定。
