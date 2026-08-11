# Contract v249：幸运、易疲劳与魔法无能

本批从 RFB `master` 的 `object2.c`、`effects.c`、`dungeon.c`、`py_throw.c`、
`cmd1.c` 与 `devices.c` 落地四项随机变异，并把随机候选覆盖推进至 103/104。

- Good/Bad Luck 由唯一 `LuckBias` 查询驱动。现有三档 loot 权重映射为累计的
  good/great 概率：好运增加 5/2 个百分点；厄运令 good 降低 5 个百分点、great
  降低四分之一，并按原版降低装备附魔与设备生成使用的有效深度。永久属性在 18
  以下使用 70/75/80 的好运/普通/厄运阈值，属性 17 固定为 58。
- Easy Tiring 在每次近战攻击与成功投掷后调用同一疲劳事务。它复用已有
  `minorSlow`，新增 `minorSlowEnergy`，按 `1 / (16 - minorSlow)` 触发，并在世界
  tick 中按角色再生率恢复。新字段进入存档与 State Hash。
- Impotence 只在共享设备检定入口生效：staff/rod 的设备技能减 10；加速效果以及
  标记为 fireball/quickness 的设备或词缀再减 20；wand 不受影响。当前角色模型
  没有性别，源账本继续记录 `exclude-sex:female`，运行时不为单个变异新增性别状态。

项目尚无伪鉴定、Liquid Logrus、随机神器与完整随机诅咒生成器；本批不为不存在的
系统制造占位消费者。相应规则应在那些正式系统进入项目时接入同一 `LuckBias` 查询。

兼容边界：Protocol 1.167、State Hash Schema v84、Contract v249、内容包 1.241.0；
save 容器仍为 v1，旧开发存档不兼容。`minorSlowEnergy` 改变所有状态哈希，因此 21
个 active fixture 全量刷新。E2E 不在本批范围内。
