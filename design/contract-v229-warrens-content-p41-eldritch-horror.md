# Contract v229：P41 妖鬼与真实理智冲击

状态：已实现。协议 `1.158`，内容包 `1.225.0`，State Hash Schema `v78`，
active baseline `contract-v229`；save 容器保持 v1，不兼容旧开发存档。

## 权威来源与内容范围

- 怪物记录固定读取 RFB `master:lib/edit/r_info.txt` 的索引 `327`。
- 中文显示名与描述严格使用权威中文表中的“妖鬼”及对应文本。
- 严格同步清单新增 `demo.actor.ghast`，总计 369 条；正式包增至 434 actors，
  abilities 保持 174。
- `ELDRITCH_HORROR` 映射为窄标签 `eldritch-horror`；不以普通 `scare` 近似，
  也不建立独立理智资源或新的通用 effect。

## 触发与后果

- 只在敌对、存活的恐怖怪物从不可见转为可见时检查。宠物和友善怪物不触发；
  已有幻觉会产生滑稽观感，并可能延长幻觉，但不会标记为已经冲击。
- 基础 power 为怪物等级一半；Unique 翻倍，非 Unique 的群体怪减半。先通过
  power 百分比门和玩家 `savingThrowSkill - power` 豁免；已经成功触发过的同一
  实例还需额外通过 `1/5` 门。
- 恶魔免疫；不死族再按 `25 + 玩家等级` 豁免；Weird Mind 通过既有状态免疫表
  在任何 RNG 前直接免疫。
- 后果链依次尝试心智冲击、智力/感知/魅力损伤、脑击、当前层地图失忆、完全
  抵抗及永久疯狂。实现复用既有混乱、幻觉、麻痹、属性 sustain、地图知识、
  `moronic`、`cowardice` 和 `hallucination` 变异消费者。

## 持久化与确定性

- `ActorSaveDto.eldritchHorrorTriggered` 只记录实例是否已经造成过真实冲击；
  false 省略，载入默认 false。该字段同时进入 State Hash Schema v78。
- `monster.eldritch-horror` 事件携带来源、power 和稳定 outcome；Web 只负责按
  当前语言本地化，不自行解释或重算理智结果。
- 聚焦测试固定首次与重触发 RNG、状态后果、属性损伤、地图失忆、Weird Mind
  零 RNG 免疫，以及 save / State Hash 往返；471 条 active exact fixtures 统一
  刷新到 contract-v229。

正式内容 hash：
`005d3db278c595029ef2a65e8f46dcd3748c303bc96681a1a513dfc24b54c43d`。
