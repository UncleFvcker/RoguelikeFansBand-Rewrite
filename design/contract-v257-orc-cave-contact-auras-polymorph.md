# Contract v257：Orc Cave 接触光环与目标变形

本批以权威 RFB `master` commit
`efd63661302866038f58d8cd2553b23e6af3bf9d` 为来源，完成 O4 的显式接触光环和
目标 `POLYMORPH`，继续复用既有伤害、豁免、变异与怪物形态事务。

## 行为边界

- `A:FIRE` 与 `A:ELEC` 保留原版显式伤害骰，分别进入火焰与闪电抗性管线。
- `A:CAUSE_2` 先掷接触伤害，再执行施法者等级对玩家豁免；失败后造成 curse
  伤害。它沿用当前 `CurseDamage` 的既定简化，不额外诅咒装备。
- `POLYMORPH` 对玩家先执行同一豁免，失败后复用 Polymorph Potion 的变异重组；
  不增加临时种族形态状态。
- 对玩家阵营召唤物，unique/guardian 不可变形；普通目标先按等级抵抗，再复用
  变色龙的地形合法候选和 HP 比例、速度、抗性、施法状态刷新，直接替换实际 actor
  kind，同时保留实体 ID、召唤归属和控制者。

## 内容与兼容边界

Kharis the Powerslave、Hisser 与 Dokkaebi 进入 Orc Cave；Flaming crow 作为全局
内容保留其山地、荒地、林地和专属地牢分配，不错误添加 `orc-cave` 标签。审计变为
194 imported、184 selected、14 blocked、28 excluded、1 guardian。

内容包升级到 1.248.0，共 611 actors、271 abilities 和 13 loot tables；严格同步
564 条，内容 hash 为
`3f31646c59caea5c6959112540cb820110a17000cafd0c763b14b9bd058973ba`。协议升至
1.170，State Hash Schema 保持 v85，save 容器保持 v1。基线推进到
contract-v257，继续保留 22 个聚焦 exact fixture，不恢复旧 E2E。
