# Contract v96：内容层抗性档与旧版抗性旗标导入

状态：历史 baseline；当前 active baseline 见 [contract-v97](contract-v97-psionic-family.md)。协议 1.96 沿用（无 DTO 变更：怪物抗性只进存档，不进快照——知识边界保持）；内容包 1.87.0（content hash `f1fba31216da594e34b36b23bdf4570b46a934c7360ad0d66e01f1284529a9f2`）；save 容器 v1；实体 `resistances` 早已是权威存档字段，state hash 沿用 Schema v40。该 baseline 共 310 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

r_info 抗性旗标共 **4043 实例**：`RES_X`（19 种元素，POIS 516/COLD 476/FIRE 425/ELEC 336/ACID 311/NETH 245/DARK 198/WATE 112/CHAO 97/DISE 86/NEXU 71/SOUN 71/LITE 67/SHAR 67/PLAS 61/TIME 52/GRAV 33/INER 29/DISI 19）、`IM_X`（五基础元素免疫，POIS 164/FIRE 57/COLD 44/ELEC 25/ACID 24）、`HURT_X`（弱点，LITE 117/FIRE 24/COLD 11）、`RES_ALL` 4。P39 的 28 类元素就位后，本轮补齐「怪物为何抗它」的内容通道。

不映射（留缺口）：RES_TELE 238（传送抗性，非伤害类型，等位移抗性机制）、RES_WALL 34（语义待核）、HURT_ROCK 29（削岩弱点，无对应伤害类型）。玩家侧抗性来源（种族/装备）不在本轮。

## 2. 内容格式（1.87.0）

`ActorDefinition` 新增 `resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>`（serde 默认空映射；JSON 形如 `"resistances": {"electricity": "resistant", "fire": "immune", "cold": "vulnerable"}`）。新枚举 `ActorResistanceLevel { vulnerable, resistant, strong, immune }`——normal 以缺省表达。既有内容零变更、零校验新规则（类型与档位均为封闭枚举，serde 拒绝未知值）。

demo 接入：新增 `demo.actor.slag-crawler`（熔渣爬虫，glyph "c"，标签 `slag`——刻意避开 elemental/caster/echo 等既有召唤候选标签，v95 场景选择骰零扰动），声明 `{electricity: resistant, fire: immune, cold: vulnerable}`；自带 kin 式固定召唤 `demo.ability.slag-call`（summon 自身 kind、1 只、半径 2、时长 5）——同时为导入器 S_KIN 形态提供 demo 原生范例。

## 3. 执行与权威边界

- **生成盖章**：所有从定义构造实体的路径（楼层生成、初始世界实体、固定/类别召唤、玩家召唤）在构造后以 `definition_resistance_profile` 盖章内容抗性；零 RNG。
- **存档权威**：实体抗性一直是存档字段——读档以存档为准（旧存档的怪物保持空抗性，不回溯重盖）；fixture `entityEffects.resistances` 仍是显式覆盖列表，语义不变。
- 减免算术零改动：既有 `resolve_armored_damage`（护甲仅物理 + 中性五档百分比）自 v9 起已被单测覆盖，本轮只是让内容真正声明档位。
- 快照不暴露怪物抗性（玩家知识另属怪物回忆系统）；smart caster 的 observed_player_resistances 与本轮无关。

## 4. 协议与事件

零变更（协议 1.96 沿用）。伤害事件里抗性作用只体现在 applied 数值上，与既有形状一致。

## 5. 导入器映射（随后小步）

`RES_X → resistant`、`IM_X → immune`、`HURT_X → vulnerable`（按 RES→IM→HURT 次序应用，后者覆盖前者）；后缀表 POIS/COLD/FIRE/ELEC/ACID/NETH/DARK/WATE/CHAO/DISE/NEXU/SOUN/LITE/SHAR/PLAS/TIME/GRAV/INER/DISI → 对应类型；`RES_ALL` 展开为全部非物理类型 resistant（4 只）。已映射旗标从 unmappedMonsterFlags 计数中移除，RES_TELE/RES_WALL/HURT_ROCK 继续留在缺口。实测收割：1023 只导入怪物获得抗性档、3842 条条目（RES_TELE/WALL/HURT_ROCK 排除，RES_ALL 4 只展开）。

## 6. 契约场景（v96）

迁移 308 条（预期零语义漂移——新增字段带 serde 默认，既有内容序列化不变）后新增 309-310 共 2 条：

- 309 盖章+减免全链：entityEffects 把既有实体转为 slag-crawler → 等待其 kin 召唤（召唤路径盖章内容抗性，随 saveRoundTrip/stateHash 入约）→ 玩家（scholar 预置已学 resonant-bolt）电弹命中召唤体，applied = raw 减半实录；
- 310 显式覆盖优先：entityEffects 显式 `[{electricity: immune}]` 的爬虫吃电弹，applied = 0，实证覆盖列表仍胜过内容缺省。

生成路径盖章由核心单元测试补充覆盖（corridor kin 召唤 → 断言召唤体三档抗性与定义一致）。

## 7. 验证

常规全套 + `migrate-baseline` 零漂移核对 + 新场景 `refresh` 人工审阅；clippy 单跑验退出码；版本件套（pack 1.87.0 / content.lock / BUILT_IN+PREVIOUS / README；协议不动）；本地桌面 E2E（contentVisualCount 75→76）。
