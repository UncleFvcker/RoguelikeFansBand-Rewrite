# Contract v107：Death 第四册与生命/形态高级效果

## 范围

Contract v107 完成 Death 第四册 `[Necronomicon]` 的八个槽位。协议为 `1.107`，demo 内容包为 `1.98.0`，state hash Schema 为 `46`，active baseline 包含 365 条 exact fixtures、零 waiver。内置内容 hash 为 `d8bdbdd4d4e85862a97229c279a874668b9b1d3ce9035aa6f17a11cff7b3af80`。

## 通用内容与协议表面

- 目标协议增加稳定物品实例目标；能力可在消耗资源和施法 RNG 前验证物品是否由玩家携带。
- `DeathRay` 只作用于活体；普通目标进行目标等级与施法者 power 的确定性对抗，unique 先通过独立的 1/666 门，再进入等级对抗。成功时复用既有击杀、经验、任务和掉落事务。
- `SummonCategory` 支持等级切换类别、敌对概率、敌友群体概率与群体数量骰；unique 只允许出现在显式许可的敌对结果中，`guardian` 不进入类别池。
- `IdentifyItem` 在普通鉴定与完整鉴定之间进行显式 power 检定，结果写入既有存档级物品实例知识。
- `ApplyStatus` 可临时授予 Race、穿墙和入伤比例。临时 Race 参与属性、抗性、技能和资源上限投影，但不替换持久构筑 Race、不改身体槽，也不写回持久技能进度。
- 玩家成长保存历史最高经验 `maximumExperience` 和生命力 `lifeForce`；`RestoreVitality` 把当前经验恢复到历史最高值并恢复生命力。
- `Genocide` 增加玩家周围半径范围；只处理范围内普通怪物，unique 保留并进入抵抗列表。
- Wraithform 的入伤比例作用于怪物伤害和状态伤害；穿墙期间玩家可进入不可行走格，到期不传送、不改图，玩家可以留在墙内继续处理后续行动。

## Death 第四册

| 槽位 | Ability | v107 行为 |
| --- | --- | --- |
| 24 | Death Ray | living-only 即死；普通目标等级对抗，unique 先过 1/666 门 |
| 25 | Raise Dead | 低等级召唤 undead；48 级切换 high-undead，并按原版形态决定敌友、单体/群体和敌对 unique |
| 26 | Esoteria | 以物品实例为目标；普通鉴定或按等级 power 检定完整鉴定 |
| 27 | Vampiric Transformation | 临时覆盖 Vampire Lord Race 的属性、技能和抗性，保持原身体槽与持久构筑 |
| 28 | Restore Life | 当前经验恢复到历史最高经验，生命力恢复为 1000 |
| 29 | Mass Genocide | 半径 20 的 nearby Genocide；按 prorated 曲线缩放 power |
| 30 | Hellfire | nether ball；伤害加值与半径按 prorated 曲线增长 |
| 31 | Wraithform | 随机持续时间内穿墙并把受到的伤害降为 50% |

40 级投影按既有 RFB prorated 曲线固定：Esoteria power 30、Vampiric Transformation 基础/骰面 25、Mass Genocide power 92、Hellfire 加值 373/半径 5、Wraithform 基础/骰面 14。

## Legacy 导入结果

固定 legacy commit 的真实导入现在生成四本 Death 物理法书、32 个玩家 abilities、12 个运行时 casting profiles 和 384 条逐职业参数覆盖/映射行。Death 效果缺口由 192 降至 96；第四本实体法书绑定 `tval=100/sval=3`。

本地 legacy 内容包严格编译通过：1260 abilities、4 ability books、937 items、128 affixes，content hash 为 `0eebba813045b71472c720ed67735233730a594a59a971e935d410eda90c58da`。

## Fixtures 与兼容性

- 354：40/50/100 级第四册投影；
- 355/356：Death Ray 的非活体拒绝与活体击杀；
- 357/358：Raise Dead 的基础类别、升级类别、敌对群体和 unique 边界；
- 359：Esoteria 的携带物品目标、鉴定与知识保持；
- 360：临时 Race 投影、身体槽保持和 save round-trip；
- 361：Restore Life 精确恢复经验 `500 -> 900` 与生命力 `125 -> 1000`；
- 362：Mass Genocide 的半径过滤、unique 抵抗和确定性；
- 363：Hellfire 的 prorated 伤害、半径和范围结算；
- 364：Wraithform 的怪物法术半伤、穿墙、墙内到期和回档；
- 365：旧 save 缺少第四册新增进度/状态字段时按默认值载入，零补抽 RNG。

施法 fixture 可显式启用 `debugAbilityCastsSucceed`：失败率在该场景中视为 0，但仍消耗资源、推进正式施法流程并保留效果 RNG；该开关不进入正式存档或权威状态。

旧正式 demo hash `5e6e5f4ee9b83eb8d80e05c8aa893bd8d19c1db1bdd18c97fe3e120fd823a88c` 已加入兼容列表。缺失 `maximumExperience` 的旧进度以当前经验迁移，缺失 `lifeForce` 默认 1000；状态的新 Race/穿墙/入伤字段按无覆盖、不可穿墙和 100% 入伤迁移，不补状态、不推进 RNG。

## 后续候选

P58 应重新按全领域 `playerSpellEffectGaps` 与设备/消耗品 `itemBehaviorGaps` 的真实覆盖收益排序，不再默认沿 Death 继续逐册推进。Death/necromancy 当前各剩 96 条效果缺口；Invoke Spirits 的 actor polymorph、line light、earthquake 和 destroy area 仍应随对应通用系统逐项替换 `NoOp`。
