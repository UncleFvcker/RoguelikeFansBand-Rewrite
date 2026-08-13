# contract-v301：Human 主动与跨系统半神天赋

本批以 `D:/codex/Frogcomposband` 的 `master` Git ref（审计提交
`efd63661302866038f58d8cd2553b23e6af3bf9d`）为权威来源，完成六项原版半神天赋，并把
Human 20 级奖励绑定到当前已经闭环的 20 项候选。

## 内容身份

本批不新增 item、actor、mutation、material 或 affix ID。新增两对能力身份：

- `rfb.ability.mutation.peerless-tracker`
- `rfb.ability-program.mutation.peerless-tracker`
- `rfb.ability.mutation.fantastic-frenzy`
- `rfb.ability-program.mutation.fantastic-frenzy`

Human 候选严格来自 `master:src/mut.c::mut_demigod_pred`。当前只展示已经有真实消费者的
20 项；`ambidextrous`、`speed-reader`、`black-marketeer`、`tread-softly`、
`inspired-smithing`、`strong-mind` 与 `astral-guide` 继续隐藏，不能用相近效果冒充。

## 规则闭环

- 怪物新增 0–100 的权威怒气。远距离玩家法术与投射物伤害分别使用原版
  `mon_anger_spell` / `mon_anger_shoot` 公式；怒气提高下一次施法频率，成功施法后清零。
  “隐秘施法”和“无双狙击手”分别关闭对应来源。
- “闪避”使喷吐、火箭和投石伤害按一次 `1d10` 降低 11%–20%，并在地震选中玩家格后
  提供 50% 的压砸规避。
- “个人崇拜”只处理敌方新召唤实体：先执行 1/2 门槛，再按玩家等级、魅力和怪物
  Unique 修正进行两次原版保存检定，依次转为友好和宠物。
- “无双追踪者”在 20 级以 WIS、25 HP、失败参数 40 激活，复用映射与探测事务。
- “奇妙狂乱”在 40 级以 STR、50 SP 或 HP、失败参数 80 激活“大屠杀”，复用普通近战
  resolver 攻击八个相邻格。普通近战提前击杀时按已使用/可用攻击数保留剩余行动能量。

## 契约与版本

- 内容包：`1.317.0`
- content hash：`b27d385635fe09ef107ca2dd4e7fe6475d58e7e3320893e899246920779f5cb2`
- Protocol：`1.200`
- State Hash Schema：`v99`
- save 容器：`v1`，`ActorSaveDto.anger` 与 `friendly` 为必填；不兼容旧开发存档
- active baseline：`contract-v301`

按当前分支验收约定，本批只运行新增聚焦测试、生成物检查、内容锁校验、变异审计与 Web
类型检查。由于 actor 权威状态和协议投影发生变化，26 条 active fixture 的刷新及全量 replay
留到用户明确要求合并验收时执行；不得把尚未复验的 fixture 描述为零漂移。
