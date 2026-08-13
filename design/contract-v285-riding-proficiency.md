# Contract v285：骑术熟练度

状态：已实现。Protocol `1.189`，State Hash Schema `v94`，内容包 `1.298.0`，save v1。

## 内容与权威状态

- `ClassDefinition.ridingProficiency` 必填保存 RFB `master:s_info.txt` 的
  `S:2:start:max`；校验 `initial <= maximum <= 8000`。
- 当前四职业为 Warrior `0/6000`、High-Mage `0/0`、Archer `0/4000`、Paladin
  `0/6000`。导入器保留所有职业的完整杂项熟练度行；Cavalry 来源值为 `2000/8000`。
- `CharacterProgress.ridingProficiency` 保存当前值。新角色从职业 initial 开始；读档拒绝
  缺字段、低于 initial、超过 maximum 或无职业时非零的状态。

## 规则

- 等级边界独立为 `0/2000/4000/6000/8000`，复用通用等级枚举但不复用武器阈值。
- 骑乘成功范围为 `riding / 50 + level / 2 + 20`。
- 骑乘近战每次攻击事务按目标等级与坐骑等级计算原版增量；骑乘射击只在弹道碰到怪物、
  门槛成立且一次 `one_in_(2)` 成功时增长 1。门槛失败或到达上限不抽 RNG。
- 非强制落马检定的原版成长公式在本契约建立规则入口；`contract-v286` 已接入落马事务，
  并保持成长发生在判定 RNG 之前。
- 每跨过一个 100 点边界发送 RFB 原版分阶段提示；前端只呈现核心投影。

## 兼容边界

`PlayerProgressSaveDto` 与公共快照新增字段，状态哈希输入结构改变，因此 Protocol 升至
1.189、State Hash Schema 升至 v94，并统一刷新全部 26 条 active fixture。save 容器仍为
v1；开发期存档不提供兼容路径。
