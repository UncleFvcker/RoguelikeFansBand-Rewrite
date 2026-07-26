# 伤害类型扩展 v1：RFB 原版元素表

状态：已实现（P39，协议 1.96）。纯枚举扩展 + 导入器映射迭代：demo 内容零变更、compiled 内容字节零变更、无新存档字段——**contract-v95 基线保持 active，无迁移**。

## 1. 范围与依据

用户约束：不自创元素，只收录 RFB 原版（FrogComposband v1.3.0.7 固定 commit `gf.h` 元素段 GF_ACID…GF_MISSILE）。在既有六类（physical/acid/electricity/fire/cold/poison）之上按 gf.h 声明序新增 22 类：

`light`（GF_LITE）、`dark`、`confusion`、`nether`、`nexus`、`sound`、`shards`、`chaos`、`disenchant`、`time`、`mana`、`gravity`、`inertia`（GF_INERT）、`plasma`、`force`、`nuke`、`disintegrate`、`storm`、`holy-fire`、`hell-fire`、`ice`、`water`。

明确不收：GF_ROCKET/METEOR/ROCK/ARROW/MISSILE（投掷物形态而非元素——ROCKET 按原版抗性表映射 `shards`，其余 physical）；GF_CAUSE_*（诅咒走豁免检定，等豁免机制归属）；GF_PSI 系（心灵族连带状态骑手，单独迭代）；GF_AIR（Frog 追加，仅 1 处 BR_AIR，留缺口）。

## 2. 实现面

- 三枚举同步扩展：`rfb-core DamageType`、`rfb-protocol DamageTypeDto`（协议 1.92→1.96 期间唯一变更，kebab 序列化）、`rfb-content ActorDamageType`；三组 From 转换按序补齐。
- 抗性语义不变：中性五档（-50/0/50/65/100）对全部类型一致；`mana` 原版不可抗——内容层无人声明抗性即事实不可抗，不做硬编码特例。护甲仅物理参与减免（既有规则，元素攻击无视 AC 与原版一致）。
- Web：`damageTypeName` 穷尽映射 +22、双语 game.ftl 文案（中文名对齐原版汉化用词：闪光/黑暗/地狱/因果混乱/去魔/时间逆转/纯粹魔力/迟钝/放射性废物/分解…）、MESSAGE_KEYS 登记。
- 无契约迁移的理由：新增枚举变体不改变既有内容的 MessagePack 字节（demo 包 hash 不变），事件流中不出现新类型（demo 无内容使用），save 无新字段；协议 1.96 仅为 DTO 联合扩容。

## 3. 导入器（同轮）

1. **近似转正**：BO_ICE cold→ice、BO_PLASMA/BR_PLASMA fire→plasma、BO_WATER/BA_WATER physical→water、BA_NUKE/BR_NUKE poison→nuke、ROCKET physical→shards；
2. **异种元素解锁**：BO_MANA/NETHER/TIME、BA_NETHER/CHAOS/DARK/LITE/MANA_STORM（后四者按原版 MSF_BALL4 半径 4）、BR_ 全部 17 种异种吐息（含 BR_CONF 别名）；
3. **blow 效果元素名**：LITE/NETHER/NEXUS/SHARDS/DISENCHANT/TIME/INERTIA/PLASMA/DISINTEGRATE/HELL_FIRE 直映（约 84 实例），VAMP/SHATTER/DRAIN_* 等效果语义仍留缺口；
4. **实测收割**：法术映射 3071→**3849**（+778），施法怪 783→**829**，未映射 2157→**1379**；剩余头部为 CAUSE 240（豁免）、S_ 特殊/字形 177、BRAIN/MIND/PSY 心灵 248、TELE 94、DARKNESS 85、DRAIN 83、AMNESIA 64、ANIM 58。

## 4. 遗留

- 怪物/玩家的内容层抗性档尚未存在（actor 定义无 resistances 字段），RES_/IM_/HURT_ 旗标导入待「抗性档导入」迭代——类型扩展先落地，抗性接上后新元素的攻防交互即自动生效；
- 心灵族（MIND_BLAST/BRAIN_SMASH/PSY 系）需状态骑手 + 豁免语义；CAUSE 诅咒族需豁免机制归属；
- demo 包暂无新元素使用者；首个用到新类型的演示内容出现时按常规走 contract 迁移。
