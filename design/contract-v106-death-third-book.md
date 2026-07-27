# Contract v106：Death 第三册与随机效果/吸血武器

## 范围

Contract v106 完成 Death 第三册 `[Black Channels]` 的八个槽位。协议为 `1.106`，demo 内容包为 `1.97.0`，state hash Schema 为 `45`，active baseline 包含 353 条 exact fixtures、零 waiver。内置内容 hash 为 `5e6e5f4ee9b83eb8d80e05c8aa893bd8d19c1db1bdd18c97fe3e120fd823a88c`。

## 通用内容与协议表面

- `ApplyStatus` 支持基础时长加骰时长，并可在状态存活期间授予属性修正、装备加值和状态免疫；这些权威字段进入存档、回放和 state hash。
- `RandomChoice` 以一次确定性掷骰选择有序阈值分支，分支可改用施法者自身作为目标。未建立的具名规则使用可观察的 `NoOp`，不会伪装成普通伤害。
- 固定种类 `Summon` 可显式生成敌对实体；敌对召唤不获得玩家 owner/controller 身份。
- `DrainLife.repeat` 逐次重新追踪方向；前一击击杀近处目标后，后续击数可以命中同一路径上的下一目标。
- `VisibleDamage` 对所有当前可见且符合类别的目标共享一次基础伤害掷骰，目标按稳定顺序结算。
- `EnchantEquippedWeapon` 把 affix 永久写入当前武器实例；affix、知识和物品实例均随存档保持。
- `vampiric` 装备 passive 只从实际持用武器生效，每次近战命中按实际伤害掷吸血量，单次近战动作总治疗上限 50 HP；其他槽位上的同名 passive 不触发吸血。
- 等级曲线新增 `linear` 与 `prorated` 形状；后者用于保留原版 `spell_power` 的非线性成长。

## Death 第三册

| 槽位 | Ability | v106 行为 |
| --- | --- | --- |
| 16 | Berserk | `25+1d25` ticks；治疗 30，最大 HP +30、防御 -10、恐惧免疫，并按等级提高近战伤害及调整原版对应技能 |
| 17 | Invoke Spirits | `1d100 + floor(level/5)` 选择 23 个阈值分支；19 个分支使用真实效果，四个未实现规则显式 `NoOp` |
| 18 | Dark Bolt | `(4 + floor(max(level-5, 0)/4))d8` dark；复用职业 bolt/beam 档案 |
| 19 | Battle Frenzy | 分别掷 Hero、Blessed 与 Haste 持续时间；三种状态独立保存并参与派生属性 |
| 20 | Vampiric Branding | 给当前装备武器永久添加 `vampiric` affix，不修改其他物品 |
| 21 | Vampirism True | 沿同一方向执行三次 `100` 点 Drain Life，每击按实际生命损失治疗 |
| 22 | Nether Wave | 对全部可见活体共享掷 `1d(3*level)` nether 伤害，非活体不进入结算 |
| 23 | Darkness Storm | 半径 4 dark ball；基础 100，加上按原版 prorated 曲线增长的最高 200 伤害 |

Invoke Spirits 当前保留四个明确缺口：actor polymorph、line light、earthquake 和 destroy area。它们在导入报告中各记录 12 行，并通过 `NoOp.reason` 保持可观察；建立对应地形/变形系统后可逐项替换，不改变其余 19 个分支的阈值。

## Legacy 导入结果

固定 legacy commit 的真实导入现在生成三本 Death 物理法书、24 个玩家 abilities、12 个运行时 casting profiles 和 288 条逐职业参数覆盖/映射行。Death 效果缺口由 288 降至 192；`BRAND_VAMP` 映射为 `vampiric` passive，第三本实体法书绑定 `tval=100/sval=2`。

本地 legacy 内容包严格编译通过：1252 abilities、3 ability books、937 items、128 affixes，content hash 为 `4c433616d3223d6a290ab0bce23f2e9d6b21578c4769eb963a2bf3d2b5d83146`。

## Fixtures 与兼容性

- 344：40/50/100 级第三册投影、线性与 prorated 缩放；
- 345：Berserk 的随机时长、治疗、派生加值和存档；
- 346/347：Invoke Spirits 的低/高确定性随机分支；
- 348：Battle Frenzy 三个独立随机状态时长；
- 349：永久 Vampiric Branding、近战吸血和 save round-trip；
- 350：Vampirism True 三次重新追踪并跨目标结算；
- 351：Nether Wave 活体过滤、可见过滤和共享伤害掷骰；
- 352：Darkness Storm 的 prorated 等级曲线与范围伤害；
- 353：旧 save 缺少新增状态/物品字段时按默认值载入，零补抽 RNG。

施法 fixture 可显式启用 `debugAbilityCastsSucceed`：失败率在该场景中视为 0，但仍消耗 Mana、抽取施法 RNG，并完整执行效果 RNG、时间、存档和 state hash；该开关不进入正式存档或权威状态。

旧正式 demo hash `26fdeb15063fa5ccc5a672cd8d2376f7ea66e7dc487fef6f1a4d5640a1050cf9` 已加入兼容列表。新增状态授予字段、永久 affix 与吸血 passive 进入 save/state hash；缺失字段的旧档使用空值迁移，不按当前内容补抽 RNG。

## 后续候选

P57 可继续盘点 Death 第四册，优先完成其真实槽位所需的高级召唤、领域效果与地形词汇；设备/消耗品效果系统仍是并列高收益候选。Invoke Spirits 的四个 `NoOp` 应随 actor polymorph、line light、earthquake 和 destroy area 通用系统逐项清零。
