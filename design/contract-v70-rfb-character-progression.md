# Contract v70: RFB Character Progression Foundation

状态：协议 1.70 / contract-v70 active baseline；内容包 1.62.0；state hash Schema v29

## 范围

v70 建立原版式角色成长的最小闭环：击杀经验、等级、出生时确定的生命成长序列、六维自然属性、装备属性修正、胜利后的等级/属性上限解锁，以及属性点分配。

- 经验与等级沿用 RFB 的 1–50 累计经验阈值；未击败最终 Boss 时等级最高 50。
- 最终 Boss `demo.actor.serpent-of-chaos` 使 campaign 从 `active` 进入 `victorious`，同时解锁等级 100 和属性 `18/820` 上限。胜利前的属性上限为 `18/220`。
- 胜利前已经积累但受等级上限阻挡的经验不会丢失；解锁时自动连续升级到经验所允许的等级。
- 六维为 Strength、Intelligence、Wisdom、Dexterity、Constitution、Charisma。自然值持久化，装备只提供 modifier；有效值在运行时按原版 18 以下逐点、18 以上每桶 +10 的规则计算并应用当前上限。

内部整数值 `3..18/xx` 以 `18 + xx` 表示，例如 `18/220 = 238`、`18/820 = 838`；协议同时暴露自然值、有效值、原版桶索引和当前原始上限，UI 负责显示 `18/xx`。

## 命令与事件

`GameCommand::IncreaseAttribute { attribute }` 消耗一个属性点，不推进世界 tick、玩家能量、回合或 RNG。没有可用点数，或该自然属性已经达到当前阶段上限时，命令仍形成确定性更新并投影 `player-attribute-increase-unavailable`。

经验由怪物定义的 `experienceValue` 在玩家确认击杀后结算。核心按固定顺序发布：

1. `player-experience-gained`；
2. 每个连续等级的 `player-level-gained`；
3. 最终守护者触发战役胜利时的 `player-level-cap-unlocked`。

属性成功提升发布 `player-attribute-increased`，包含自然值、有效值、桶索引和剩余点数。体质改变最大生命时，当前生命按旧/新最大生命比例调整，避免免费分配点数改变相对伤势。

## HP 与存档

新角色出生时使用独立的 seed 派生 RNG 生成 100 项 1–10 的每级生命增量；该 RNG 不消耗地图、战斗或掉落 RNG。`PlayerProgressSaveDto` 保存自然属性、经验、等级、最高等级、待分配点和完整生命序列，因此载入、回放和跨平台 hash 不依赖重新抽样。

缺少 `player.progress` 的 v69 及更早存档按确定性兼容迁移：六维为 13、等级 1、经验 0，后续每级生命固定增加 6；不重建地图、不推进正式 RNG。胜利/退休存档载入时会先结算已存的封顶经验，再进行完整状态校验。

正式 save 容器仍为 v1；角色成长字段属于 payload 权威状态，并进入 state hash Schema v29。装备的最终攻防、属性修正和其他派生值仍不写入存档，而由当前内容与实例重新计算。

## 确定性覆盖

内容包 1.62.0 的 hash 为 `ad6b35c6e0ae8980a74fac51ea1e6597b09559541d4a85d598284dc2cb41d7e6`；v69 hash `06c054a8c083e05b9d0396aa1076fbe2133a6a1ce5f6c32f101e5d1dabd14b70` 与此前内置 hash 继续进入迁移白名单。demo 回声护符新增 `strength +1`，用于验证自然/有效属性分离和卸装恢复。

active baseline 有 148 个 exact fixtures，零 waiver。新增 140–147 覆盖 Ember Mote 的 10 XP 升级、免费属性点命令、18/xx 桶、胜利后自动解锁到 100、`18/820` 桶上限、旧 progress 缺失迁移、装备属性修正/存档回环，以及未胜利 50 级封顶后经验保留。核心专项测试覆盖阈值、HP 序列独立 RNG、属性点扣除、campaign Boss 解锁和非法成长状态拒绝。

## 后续边界

下一纵切为完整角色创建与构筑基础（Race/Class/Personality、技能和职业成长）。v70 暂不引入法力、技能熟练、属性损伤/恢复、职业专属资源、怪物玩家种族或完整法术书；这些系统必须复用本切片的持久自然值、派生管线、事件和 save 迁移边界。
