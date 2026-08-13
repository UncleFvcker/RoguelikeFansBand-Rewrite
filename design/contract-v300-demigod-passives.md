# Contract v300：被动型半神天赋

状态：已实现。Protocol `1.199`，State Hash Schema `v98`，内容包 `1.316.0`，save v1；
active baseline 为 `contract-v300`，共 26 条 exact fixture、零 waiver。

## 契约范围

- 不屈不挠按玩家等级增加同值最大 HP。
- 狂饮药水把带 `potion` 标签物品的使用能量减半，不影响卷轴或其他物品。
- 神圣活力把共享玩家治疗事务的治疗量增加 20%；自然恢复不属于该事务。
- 恐怖巫术提供原版 `spell_power +2`，并使 STR、DEX、CON 各 -1。
- 武器多面手把所有逐武器熟练度上限提高至 8000，出生值与当前值保持不变。
- 恶魔契约在有几何视线的敌对、非宠物 actor 死亡时恢复生命；拥有可用施法资源的
  职业恢复 `level * 4 / 9` HP 与 `level * 2 / 9` 资源，其他职业恢复 `level * 2 / 3` HP。
- 恶魔之握使现有怪物装置充能吸取事务无效；免疫时既不吸取背包装置，也不回退吸取
  nutrition，且不消费装置候选 RNG。

内容模型只增加上述消费者实际使用的数值、比例和布尔字段；没有通用脚本解释器，也没有
按 mutation ID 硬编码运行时分支。此前 12 项已实现被动天赋继续由既有聚焦测试复验。

## 内容与后续边界

本批不新增 mutation、item、ability、actor、race、class、build、material 或 affix ID；
复用既有七项 `rfb.mutation.*` 身份。mutation 导入账本由 125 active / 27 blocked 推进至
132 active / 20 blocked，随机候选仍为 104。

RFB Human 20 级使用完整 `mut_demigod_pred` 候选池，其中隐秘施法、无双狙击手、左右开弓、
闪避、无双追踪者、个人崇拜、星界向导、狂暴幻想与灵感锻造等仍缺真实消费者。因此
`demo.race.rfb-human` 本批继续只配置 35 级弱点，不提前开放残缺的 20 级选择池。

## 确定性

没有新增权威状态、命令或协议投影。七项规则仅在相应变异已激活时改变派生或现有事务；
26 条 active fixture 均未持有这些变异，`verify-all` 复验零漂移，因此不刷新 assertions。
