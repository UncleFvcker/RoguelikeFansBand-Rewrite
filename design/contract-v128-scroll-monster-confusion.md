# Contract v128：Monster Confusion 卷轴

日期：2026-07-29

Contract v128 接入原版卷轴 sval 36 的 Monster Confusion。协议新增玩家准备态投影并升级到 `1.119`；demo 内容包为 `1.119.0`，save 容器保持 v1，state hash Schema 升至 `53`。active baseline 包含 431 条 exact fixtures、零 waiver，内置内容 hash 为 `757be0f1513b9cbfb2f77e08ceef8bff8ffcdb10fc7da17a0da05dbe32f908a0`。

## 1. 效果边界

内容层新增无参数、self-only 的 `prepare-confusing-strike` 物品效果。合法阅读把玩家专属权威字段 `confusingStrikeReady` 设为 true；重复阅读仍消费卷轴并保持 true，不叠层、不延长，也不建立通用 prepared-effect 枚举、on-hit trigger 或共享 Actor 字段。

miss 保留准备态。致死命中在混乱分支前结束，也保留准备态。第一个造成非致死伤害的成功近战命中先清除准备态，然后按以下顺序结算：

1. 目标具有 `rfb.status.confusion` immunity 时免疫，零效果 RNG；
2. 否则抽 `bounded(100)`，结果小于目标 actor level 时抵抗；
3. 成功时再抽 `bounded(player level)`，以 `10 + roll / 5` ticks、Extend 方式施加 confusion。

多段攻击因此只由第一个符合条件的非致死命中触发。当前规则不开放成功率、状态 ID、持续时间公式、攻击类型或触发次数等内容参数。

## 2. 协议、存档与界面

`Game` 保存玩家专属 bool；`PlayerDto` 与 `PlayerSaveDto` 镜像该字段，旧存档缺字段迁移为 false。准备态进入存档、回放和 state hash，因此协议升级到 1.119、state hash Schema 升到 53；save 容器仍为 v1。

Web 只在既有效果区域条件显示“混乱攻击已准备”，不把该 bool 伪装成带剩余 tick 的 `StatusDto`。阅读、免疫、抵抗和施加继续使用普通事件 envelope 与 Fluent 文案，没有新增 outcome DTO 或专门 E2E 场景。

## 3. 导入与契约

legacy importer 将 tval 70 / sval 36 映射为 `prepare-confusing-strike`，并把怪物 `NO_CONF` 映射为 actor `statusImmunities: ["rfb.status.confusion"]`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 20 降至 19；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `cd8e1982e33c20555019b77bec49a44fb1028e81bf54729923b5e78a7cbc1d3e`。

fixture 431 固定阅读、消费、Aware、非致死命中、两次效果 RNG、11 tick confusion、最终剩余 1 tick 和存档回读。一个窄核心组合单测覆盖 miss、致死命中、`NO_CONF` 免疫和等级抵抗；导入器既有表测试增加 sval 36 与 `NO_CONF` 行。

既有 430 条 fixture 只更新 `stateHash` 与 `saveRoundTripStateHash`。

## 4. 明确遗留

- Protection from Evil、Understanding、Inventory Protection 和其他剩余卷轴继续独立分组；
- Monster Confusion 只响应玩家近战，不提前覆盖射击、投掷、召唤物、反击或能力伤害；
- actor `statusImmunities` 当前只为真实 `NO_CONF` 提供规则输入，不建立怪物回忆或通用免疫 UI；
- Ring 无近战种族、原版 `_scroll_power`、职业特例和状态 lore 不在本轮；
- 剩余 19 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
