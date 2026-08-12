# Contract v277：原版挖掘力与地形检定

## 权威来源

- 挖掘力表与装备派生来自 RFB `master:src/tables.c` 的 `adj_str_dig`、
  `master:src/xtra1.c` 的 `_obj_digging_bonus/_equipment_digging_bonus`。
- 地形成功式、永久墙与继续挖掘条件来自 `master:src/cmd2.c` 的
  `do_cmd_tunnel_aux`；碎石、矿脉和花岗岩 power 来自 `master:lib/edit/f_info.txt`。
- 内容只读取 `D:/codex/Frogcomposband` 的 `master` Git 对象，不依赖其工作树。

## 内容与派生

- `TerrainDefinition.digging` 以 `resultTerrainId/power/resolution/veinYield` 表达
  `soft/hard/permanent` 三类地形；旧的通用 `digToTerrainId/digCheckDifficulty` 已移除。
- 新增稳定 terrain ID `demo.terrain.rubble`（`rubble / 碎石堆`）。岩浆、石英、花岗岩
  power 分别为 10、20、40；永久墙没有替换 terrain。
- `ItemDefinition.tunnelingPval` 保留原版挖掘工具 pval。Shovel、Pick、Gnomish Shovel、
  Orcish Pick 的装备贡献分别为 46、55、66、75。武器与工具各自取最大，最终仍只取一个
  最大值，不相加。
- 玩家基础挖掘力只使用 38 档力量表；职业 skill set 不再贡献挖掘力。状态上的
  `diggingSkill` 继续作为原始加值，因此狂暴的 +30 行为不变。

## 动作语义

- soft 使用 `dig > randint0(20 × power)`；hard 使用
  `dig > power + randint0(40 × power)`。hard 失败时仅 `dig > power` 可继续，soft
  失败始终可继续。
- permanent 是合法的标准耗时动作，但不抽 RNG、不改变 terrain、不可继续。
- 可挖地形上的怪物复用普通近战；地面物品不阻塞。门和其他不可挖地形仍明确拒绝。
- 前端只读取失败事件中的 `retryable` 参数恢复本次挖掘模式，没有新增自动重复状态或命令。

## 契约与版本

- 内容包升至 1.276.0，content hash 为
  `ee561b30744f44fd627805d8ed0a45eb64b21ec4c13991033b6f6254b29156b9`。
- Protocol 保持 1.179、State Hash Schema 保持 v89、save 容器保持 v1；事件沿用既有
  `GameEventDto.args`，没有新增协议字段。
- Warrior 的错误 `demo.skill.digging` skill-set 条目被移除，且相邻墙的公共 terrain
  interaction 投影发生变化，因此 24 条 active fixture 统一刷新并复验，baseline 推进为
  contract-v277，零 waiver。
- 本批不生成矿脉财宝、碎石物品或挖掘疲劳；这些属于后续挖矿提交。
