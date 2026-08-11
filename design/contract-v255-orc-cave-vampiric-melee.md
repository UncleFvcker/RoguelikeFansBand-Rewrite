# Contract v255：Orc Cave 吸血近战

本批以权威 RFB `master` 为来源，将 `VAMP` 表达为既有物理近战伤害上的窄
`vampiric` 标记。伤害不经过护甲减免，但继续经过统一的物理伤害、玩家减伤和
伤害提交事务；攻击者只按最终实际伤害恢复生命，且不超过自身生命上限。

非生命玩家仍会承受这次物理伤害，但不会为攻击者提供治疗。召唤物和敌对怪物之间
发生同类近战时复用同一结算，带 `nonliving` 标签的目标同样不提供治疗。没有增加
独立吸血状态、第二套伤害管线或协议字段。

新增 Vampiric mist、Black 和 Vampiric ixitxachitl 共 3 条 Orc Cave actor。
Vampire 还依赖尚未实现的 `UNLIFE`，因此继续留在审计账本，不用近似行为提前导入。
审计现为 189 imported、179 selected、0 direct、19 blocked、28 excluded 和
1 guardian。内容包升级到 1.246.0，共 605 actors、264 abilities 和 13 loot
tables，内容 hash 为
`9fe4504149460bb615380075d2459064eb09c958a20f4780519b4656c47fee1a`。

协议保持 1.169，State Hash Schema 保持 v85，save 容器保持 v1；active baseline
推进到 contract-v255，继续保留 22 个聚焦 exact fixture，不恢复旧 E2E。
