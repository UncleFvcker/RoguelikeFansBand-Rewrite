# Contract v250：Chaos Gift 与混沌神明

本批完成最后一个随机候选变异 Chaos Gift。数据与行为取自 RFB `master` 的
`chaoswar.c`，包含 16 位混沌神明、各自偏好属性和完整 16×20 奖励表。

- 所有新角色出生时等概率确定一位神明并保存 `chaosPatronId`；神明身份独立于角色
  当前是否拥有 Chaos Gift。
- 只有首次达到新的历史最高等级才触发。每次先以 1/6 获得随机变异，否则按原版
  等级恶性概率与神明表选择奖励；重新夺回曾达到的等级不会重复结算。
- 奖励复用现有经验、物品、属性、治疗、伤害、召唤、诅咒、毁灭、灭族与驱散
  事务。原版唯一 `Ignore` 合法无效果，其余可达奖励均执行真实行为。
- 混沌武器保留原版等级区间，从匕首逐级到混沌之刃，并获得 Chaos 词缀、额外抗性
  与附魔。17 把此前缺失的剑类从 RFB `master` 正式导入。
- Purple Gift 与 Chaos Gift 保持互斥；项目尚无 Chaos Warrior/Chaotic personality
  角色身份，因此不添加虚假的资格状态。

兼容边界：Protocol 1.168、State Hash Schema v85、Contract v250、内容包 1.242.0；
save 容器仍为 v1，旧开发存档不兼容。出生 RNG 和存档哈希改变，21 个 active
fixture 全量刷新。过期 E2E 不在本批范围内。
