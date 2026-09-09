# E5.0 护甲 Ego 权威审计

日期：2026-09-09。工作树 `D:/codex/RoguelikeFansBand-Rewrite-realms-items`，分支
`codex/realms-items`。本批完成审计和现有基础身份回填，不实现或开放护甲 Ego 池。

权威为 `D:/codex/Frogcomposband/master` 的 Git `master`，本次解析提交
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。以下原版路径和行号均指该提交的 Git 对象，
不指当前工作区文件。以后执行仍读取当时的 `master`，静态契约有变化即报错，由下一批重新审计。

## 交付和边界

- 去重 76 条，中文名全部来自 `src/ego_name_zh.inc`，unresolved 为 0；75 条非零 rarity，
  source 103 为 rarity 0。第一组 28 条，第二组 48 条，50–52 的共享定义只保留一份。
- `crates/rfb-legacy-import/src/content/armor-ego-contract.json` 锁定每条的 source index、
  原版中英文名、完整 T/W/C/F/E 数据及稳定 ID。它是审计输入，尚未物化的 71 条不创建占位 affix。
- `armor-base-contract.json` 锁定当前 38 件普通护甲的 source index、英文源名和 tval/sval。
  回填仅增加 `rfbBaseKind`，不重导旧适配物品，不改名字、数值、商店、奖励或掉落权重。
- pack `1.386.0`，357 件物品、65 个 affix；本批不改 state-hash schema 或持久字段。
  content hash：`41366794ca60ea31a1c9f5f062971407eb03987a4af92388ceff04f2e3489e73`。

## 共用规则：每条都需要，不能用静态 F/C 顶值代替

1. `src/ego.c::ego_choose_type` 按 source 顺序和等级调整 rarity 加权。W 的 min/max 是权重惩罚，
   不是硬等级门槛；rarity 0 不进入标准池。类型内先选择、再拒绝不合法 subtype、再重选；
   必须保留拒绝所消耗的 RNG。表中的“限制”叠加 T 类型限制。
2. `src/ego.c:3595` 的 `obj_create_armor` 在调度前就掷 `toac1=1d5+m_bonus(5,L)`、
   `toac2=m_bonus(10,L)`，即使 Craft 也不能省去这些 RNG。普通生成保留 power、诅咒质量、
   `_check_rand_art` 和神器入口顺序。只有普通 Robe、L>=30 且 1/7 时进入 ROBE 专用分支
   （类型 token 为 `ROBE`）；否则走 BODY_ARMOR/随机神器调度。强制 Ego/Craft 的神器禁用语义沿用原版。
3. `src/ego.c:3809` `ego_finalize` 顺序是生成诅咒、ONE_SUSTAIN/XTRA_*、C 的命中/伤害/AC
   独立有符号骰、pval、后续增幅、device-power pval 限制，再处理 power=-2 的 curse_object。
   XTRA_H_RES 不只一次：第二次条件为 `randint1(L)>60`。属性掷骰允许重复，不能改成无放回抽取。
4. 共用 pval 是基础 pval 加 1d(max)，不能把每个属性独立掷骰。Bat/Cowardice/Aman 改成重设
   1d(max)，Elven Cloak 再加 randint0(2)；Hero 上限 3；Augmentation 超过 4 后改 4+randint0(2)。
   Berserker 手套改 1d2，再 1/15 加 1。带 SPEED 的 Tomte 是 randint0(3) 后至少加 1，
   再按 `one_in(max(7,(100-L)/6)) && !BAD_LUCK` 循环；短路和 BAD_LUCK 的读取顺序也属于契约。
   Elvenkind 靴 L>70 加 1d5 AC 且 while(1/3) 增 pval；Aman L>80 while(1/4) 增；
   Magi/Lordliness/Might L>80 以 1/5 增。DEVICE_POWER 的 pval>=3 最后改 2，再 1/30 加 1。
5. `ACTIVATION_CHANCE` 是普通种族 5、Monster Ring 种族 2（`src/ego.c:454`），以下用 **A** 表示。
   不得把所有随机激活写死为 1/5。`GREAT_OBJ=8` 来自 `src/defines.h:397`。
6. `src/ego.c:471` 的显式 choices 列表交给 `effect_add_random_p`，按 devices 表 rarity 加权，
   不是列表等概率。`src/devices.c:2496` 起两种选择均按表顺序、权重 `max(255/rarity,1)`；
   bias 选择还过滤 `effect.level < object_level/3` 和 bias 位，显式列表选择没有此等级过滤。
   `_add_index` 写 type/power/difficulty/cost，extra=0、timeout=0；必须复用/扩展 E3 激活 helper，
   不可以凭 ability 名称相似就视为消费者已验收。

## 已有字段与真实消费者缺口

下表中的“待接”均阻塞相应 E5 类型开放。现有种族、突变或状态已具备的能力，应扩充其来源聚合，
不要另建护甲专用战斗系统。每条的静态未映射 flag 另列在逐 index 表中；已映射也不代表动态分支已实现。

| 机制 | 原版证据 | 当前 owner 与下一步 |
| --- | --- | --- |
| 抗性、易伤、sustain、ESP、free action、六属性、速度、slay/brand | `equip.c`；`ego.c:2548–3594,3809` | 复用 `AffixPropertyBundleDefinition`、`rolledAffixes`、`player_stats.rs`、战斗聚合；补各 source 的顺序和共享 pval，不增加平行属性容器 |
| 附魔和诅咒 | `ego_finalize`；`get_curse`/`curse_object` | 复用 `enchantments`/`RolledAffixState`/curse；Berserker 是覆盖附魔，普通增量接口需按已有附魔求差。RANDOM_CURSE2、永久诅咒等需逐项消费者测试 |
| 反射 | `equip.c:1788` | `player_stats.rs::player_reflects_bolts` 只读基础物品/种族/状态；affix 和 rolled property 缺表达与聚合。盾牌 62、共享 52、龙甲 89、Dwarven/Emu 动态分支依赖 |
| 重量、基础 AC | `ego.c:2877,3096,3226,3240,3575` | 重量 helper `item_weight_tenths_pound(kind_id)` 和施法负重只读定义，缺实例重量与消费者。基础 AC 增量优先复用 `rolledAffixes.properties.modifiers.defense`，与 `enchantments.to_armor` 分开；不要仅因重量需要扩展就新增 AC 持久字段。Twilight 的基础 kind/AC 重设另按下行处理 |
| Twilight 基础 kind 切换 | `ego.c:2716` | 需要目标 Yoi yami Robe 的权威物品定义；现有 `kind_id` 可表示更换，但 Ego 原子结果尚无该操作。按原版只改 k_idx/sval/ac/to_a，不因重建物品而丢掉其它实例状态 |
| 元素/碎片/复仇光环 | `equip.c:1789–1793`、`melee1.c:925` | `monster_combat.rs::player_elemental_contact_aura_sources` 已处理突变和状态，但不读装备；可复用接触流程。碎片与 AURA_REVENGE 的反击不是等同元素伤害，也不是现有 vengeance 伤害反弹 |
| DEC_MANA | `equip.c:1734`、`spells.c:1000`、`object1.c:378` | `player_abilities.rs::ability_effective_resource_cost` 只有熟练度折算；待接装备减耗。Wizardstaff 基础 DEC_MANA 和 Witch 122 共用，Wizardstaff 完成后再开放 Arcane |
| EASY_SPELL | `equip.c:1735`、`spells.c:1026,1053`、`devices.c:167,3255` | 待接施法失败率及原版装置使用的相关路径；并保留 object1 的有效 flag 规则，不只是显示技能加成 |
| SPELL_CAP / REGEN_MANA | `equip.c:1739`、`xtra1.c:3459` | `player_abilities.rs` 法力上限已有职业和种族容量，缺 affix pval；恢复路径需补装备来源。不能混同增加可学习法术数 |
| SPELL_POWER / DEVICE_POWER | `equip.c`、devices 使用路径 | 已有 `modifiers.spell_power_bonus` 和 `equipment_bonuses.device_power` 聚合；复用并测试共享 pval、正负属性及 finalize 限幅 |
| LORE2 自动鉴定 | `equip.c:1091,1547` | `mutations.rs::player_auto_identifies_items` 目前仅读突变，`game/mod.rs` 已有动作后自动鉴定入口；扩装备来源，测试拾取/装备及知识更新 |
| NO_MAGIC / MAGIC_RESISTANCE | `equip.c:1794`；法术调用/抗魔消费者 | `abilities/casting.rs` 有反魔法状态拒绝，词条来源待接；MAGIC_RESISTANCE 不能错误映成普通元素抗性 |
| DUAL_WIELDING / BLOWS / SHOTS | `equip.c:1806`、`xtra1.c:4529,4777,4825` | Genji 是真实双持规则，不是固定命中；护甲额外攻击/射击需走玩家近战/远程节奏，不能依赖仅写武器实例 traits 的旧接口 |
| LITE / DARKNESS / NIGHT_VISION、TELEPORT 等 | `equip.c` 和相应世界/视野/诅咒消费者 | 按逐项 flag 扩现有光照、传送及持续负面效果来源；契约里的 HIDE_TYPE/SHOW_MODS/AWARE 是显示/知识语义，不是可随意丢弃的战斗加成 |
| 龙甲 activation | `ego.c:2575`、`devices.c`、k_info E: | 实例已有 activation/charges；Breath 需复制基础激活后冷却减半、extra 伤害翻倍，Lore/Death 替换而非叠加。当前多彩龙甲已有 250 伤害/700 恢复，Breath 应为 500/350。禁止更改全局 effect program 来影响普通龙甲 |

## 稳定 ID 与旧适配处理

正式内容已存在的 5 个 ID：50 `rfb-legacy.affix.protection`、56 `rfb-legacy.affix.seeing`、
72 `rfb-legacy.affix.olog-hai`、126 `rfb-legacy.affix.magi-headgear`、138 `rfb-legacy.affix.wizard-gloves`。
50/72 尚无完整 rfbEgo 身份，其余已有身份也不表示运行时分支完成。后续就地升级这些 ID，保留固定引用。
64 的候选 ID 固定为 `rfb-legacy.affix.endurance-64`，避免撞 source 184 弹药的 `endurance`。
所有其它 ID 见静态契约和下表，都是待实现的名字预留，不在本批写入正式 affix 目录。

| 已有 source | 当前适配与权威分支的差距 |
| --- | --- |
| 50 | 现有 rollGroups 只有 1d10 defense，缺身体/盾/披风/头盔的 1/3 额外 AC 分支；base-items、warrior、mage、paladin、dwarf 的显式旧池仍引用它 |
| 56 | 固定 searchSkill=6，缺 1d6 共用 pval 和冠/盔不同概率的 ESP |
| 72 | 固定 STR=4/-INT=4/defense=10/meleeDamage=7 和一次简化高抗；缺原版 pval、C 骰、动态 CON/-潜行与高抗二次条件。已有 berserk 效果可复用，现恢复 intervalTicks=50；E3 标准换算是源 cost*10，后续要核对并改为原版 50 回合。Troll Cave 固定奖励仍引用此 ID |
| 126 | 固定 INT=3，额外能力被压成盲抗/混乱抗/see invisible 三选一；缺完整静态随机奖励、冠/尖帽差异和随机激活 |
| 138 | 已有三档 INT/deviceSkill 共用 roll，但缺负 C 骰、负 STR/CON、随机抗性、DEVICE_POWER 分支 |

## 76 条逐 index 清单

类型 token 与 flags 保留原版拼写；C 为命中/伤害/AC/pval 最大值，W 为最小等级/最高等级/rarity，`—` 表示无最高等级。
每一行都需先实现共用 finalize；“静态未映射”来自当前 importer，只是补查入口，具体消费者缺口以上表为准。
ID 列统一省略 `rfb-legacy.affix.` 前缀。

| index / 权威中文名 | T；W；C | 稳定 ID | 限制、动态分支和激活 | 静态未映射 |
| --- | --- | --- | --- | --- |
| 50 保护之 | BODY_ARMOR,SHIELD,CLOAK,HELMET,GLOVES,BOOTS；0/30/2；0,0,10,0 | `protection` | 各 T；1/3 加 m_bonus(10,L) AC；gloves/boots 的类型分支没有调用此 helper，保留该差异。 | — |
| 51 元素保护之 | BODY_ARMOR,SHIELD,CLOAK；0/60/3；0,0,8,0 | `elemental-protection` | 身体 1+m_bonus(6,L)，盾/披风 1+m_bonus(5,L) 次元素抗性；L>20 时 1/4 毒抗；1/A ELEMENTAL 激活。 | — |
| 52 天界保护之 | BODY_ARMOR,SHIELD,CROWN；50/—/4；0,0,10,0 | `celestial-protection` | 身体 2+m_bonus(3,L)，盾/冠 1+m_bonus(3,L) 次高抗；1/7 hold life；盾 1/5 反射，冠无身体奖励。身体叠加 AC/免麻/缓消化/反射，低基础 AC 的 supercharge 按 ego.c:2775–2831 短路和六分支顺序实现，不重掷基础 AC。 | — |
| 53 精灵的 | BODY_ARMOR,SHIELD；40/—/8；0,0,10,3 | `elvenkind` | 1/4 DEX，其中 1/3 -STR；只有身体 L>60 时 1/7 SPEED；共享 finalize 高抗。 | XTRA_H_RES |
| 54 潜行之 | CLOAK,BOOTS；0/40/2；0,0,0,3 | `stealth` | 无独立分支；共用 pval/知识与静态 flag。 | AWARE |
| 55 行动自如之 | GLOVES,BOOTS；0/40/2；0,0,0,0 | `free-action` | 无独立分支；FREE_ACT/AWARE。 | AWARE |
| 56 视觉之 | HELMET,CROWN；0/60/2；0,0,0,6 | `seeing` | 冠 1/3、盔 1/7 才抽 ESP；其中 1/2 strong，否则 weak(FALSE)。允许尖帽。 | — |
| 60 (矮人的) | SHIELD；10/—/3；5,5,12,0 | `dwarven` | 盾：拒绝两种皮盾、龙盾、镜盾；重量 base*2/3、基础 AC+4，1/4 sustain CON。 | — |
| 61 (兽人的) | SHIELD；10/—/3；3,6,10,3 | `orcish` | 盾：拒绝龙盾/镜盾；静态与 XTRA_E_RES。 | XTRA_E_RES |
| 62 反射之 | SHIELD；0/—/6；0,0,5,0 | `reflection` | 盾：拒绝镜盾；REFLECT 消费者待接。 | AWARE, REFLECT |
| 63 昼夜之 | SHIELD；0/—/6；0,0,5,0 | `night-and-day` | 盾：静态明暗抗性。 | — |
| 64 耐力之 | SHIELD；0/70/4；0,0,0,5 | `endurance-64` | 盾：1/3 sustain CON；ID 与弹药 184 消歧。 | — |
| 70 (矮人的) | BODY_ARMOR；30/—/3；0,0,15,3 | `dwarven-70` | 只限硬甲且排除锈链甲；重量 base*2/3、基础 AC+5。低等级负面/高等级潜行速度、命伤、pval、level 循环奖励和极小概率重量 base/2；L>60 时 1/(2A) WARRIOR 激活。完整短路/循环见 ego.c:2872–2958。 | — |
| 71 强兽人的 | BODY_ARMOR；30/—/8；5,5,5,3 | `the-uruk-hai` | 只限硬甲；1/4 -STEALTH。 | XTRA_H_RES |
| 72 食人妖的 | BODY_ARMOR；30/—/8；0,7,10,4 | `olog-hai` | 只限硬甲；独立 1/4 CON、1/4 -STEALTH；固定 BERSERK。 E: `BERSERK:10:50`。 | XTRA_H_RES |
| 73 恶魔的 | BODY_ARMOR；60/—/8；0,7,10,4 | `the-demon` | 只限硬甲；L>66 时 1/6 SPEED；1/A DEMON 激活。 | AURA_FIRE, XTRA_H_RES |
| 74 恶魔领主的 | BODY_ARMOR；90/—/64；0,15,15,5 | `the-demon-lord` | 只限硬甲；同 73 的动态分支；静态/诅咒不同。 | AGGRAVATE, AURA_FIRE, TELEPATHY, TY_CURSE, XTRA_H_RES |
| 75 小恶魔的 | BODY_ARMOR；18/—/7；-5,-5,5,3 | `the-imp` | 拒绝硬甲；循环至至少一个 pval flag，DEX/-WIS 各 1/2，INT/-STR 各 1/7；再有条件潜行。L>55 的幽冥抗性、L>36 或额外等级随机门的速度；1/(2A) DEMON 激活，1/2 命中+2；不能合并短路。 | XTRA_H_RES |
| 76 增幅之 | BODY_ARMOR；50/—/10；0,0,15,4 | `augmentation` | 无独立分支；finalize 共用 pval 超过 4 时重设为 4+randint0(2)。 | — |
| 77 鸸鹋领主的 | BODY_ARMOR；30/—/64；0,0,0,4 | `the-emu-lord` | 只限 Filthy Rag，还要通过 1/2 随机接受；m_bonus AC/命伤，多次元素或高抗、弱 ESP、六属性/sustain/速度/反射等独立等级检查；见 ego.c:3009–3044。 | — |
| 80 恒久之 | ROBE；30/—/1；0,0,10,0 | `permanence` | 仅 ROBE 专用入口；静态全部 sustain 与 XTRA_H_RES。 | XTRA_H_RES |
| 81 暮光的 | ROBE；50/—/10；0,0,0,0 | `the-twilight` | 仅 ROBE 专用入口；改 k_idx/sval 为 Yoi yami Robe，基础 AC/to_a=0，其它实例状态保留。 | LEVITATION, REFLECT |
| 82 咒术师的 | ROBE；70/—/10；-25,-25,0,5 | `the-sorcerer` | 仅 ROBE 专用入口；额外三次高抗；静态 DEC_MANA/EASY_SPELL/SPELL_CAP/REFLECT 消费者待接。 | DEC_MANA, EASY_SPELL, REFLECT, SPELL_CAP |
| 85 (学识的) | DRAGON_ARMOR；0/—/1；0,0,0,3 | `lore` | 1/3 ESP（strong/weak 各半），1/7 MAGIC_MASTERY，1/5 LORE2；1/A 从 Lore 列表选激活并替换基础吐息。 | LORE2 |
| 86 (吐息的) | DRAGON_ARMOR；0/—/1；0,0,0,3 | `breath` | 复制基础龙甲 activation，cost 整除 2、extra*2；不能叠加第二个激活或修改公用 program。 | — |
| 87 (攻击的) | DRAGON_ARMOR；0/—/1；5,5,0,3 | `attack` | 命中/伤害各+3；1/3 fear 抗性；静态 STR。 | — |
| 88 (工匠的) | DRAGON_ARMOR；0/—/1；0,0,0,3 | `craft-88` | 无独立分支；静态 WIS/MAGIC_MASTERY，共用 pval。 | — |
| 89 (装甲的) | DRAGON_ARMOR；0/—/1；0,0,5,3 | `armor` | AC+5、1/3 再加 m_bonus(10,L)；while(1/2) sustain；独立 1/7 反射、1/7 碎片光环。 | — |
| 90 (支配的) | DRAGON_ARMOR；0/—/1；0,0,0,3 | `domination` | 无独立分支；静态 CHR/fear 抗性，共用 pval。 | — |
| 91 (圣战的) | DRAGON_ARMOR；0/—/2；5,5,0,1 | `crusade-91` | 只允许 Gold/Law DSM；1/7 BLOWS。当前正式基础物品均缺。 | — |
| 92 (死亡的) | DRAGON_ARMOR；0/—/3；0,0,0,3 | `death-92` | 只允许 Chaos DSM；1/6 Vampiric，1/A Death 列表替换吐息。当前基础物品缺。 | — |
| 95 懦弱之 | CLOAK；0/70/4；-10,-10,-10,3 | `cowardice` | 无独立分支；SPEED/畏惧易伤/-CHR；Cowardice 重设 pval 特例。 E: `TELEPORT:20:100`。 | — |
| 96 献祭之 | CLOAK；10/60/4；0,0,0,0 | `immolation` | 1/A FIRE 激活；静态火光环消费者。 | AURA_FIRE |
| 97 电力之 | CLOAK；10/60/4；0,0,0,0 | `electricity` | 1/A ELEC 激活；静态电光环消费者。 | AURA_ELEC |
| 98 冰冻之 | CLOAK；10/60/4；0,0,0,0 | `freezing` | 1/A COLD 激活；静态冷光环消费者。 | AURA_COLD |
| 99 报复之 | CLOAK；50/—/32；0,0,-20,0 | `retribution` | 独立 1/2 火/冷/电光环、1/7 碎片光环，另有静态复仇光环。 | AURA_REVENGE |
| 100 暗影之 | CLOAK；30/—/8；0,0,0,7 | `shadows` | 1/3 光易伤；静态潜行/暗抗/darkness。 | DARKNESS |
| 101 阿门洲的 | CLOAK；50/—/16；0,0,10,3 | `aman` | 无独立分支；Aman 重设 pval 特例与 XTRA_H_RES；L>80 while(1/4) 增幅。 | XTRA_H_RES |
| 102 蝙蝠的 | CLOAK；50/—/16；-7,-7,-5,5 | `the-bat` | 命中/伤害各-6；1/6 darkness；1/3 光易伤+高抗且其中 1/3 -STR；1/12 night vision；共用 pval 特例。 | LEVITATION |
| 103 戒灵的 | CLOAK；50/—/0；6,6,6,3 | `the-nazgul` | rarity=0；命中/伤害各+6；1/6 永久诅咒、1/66 冷免疫、while(1/6) 高抗；1/A NECROMANTIC 激活，另有静态重诅咒/随机诅咒/吸经验。 | DRAIN_EXP, HEAVY_CURSE, RANDOM_CURSE2, XTRA_H_RES |
| 104 英雄的 | CLOAK；50/—/32；4,4,2,2 | `the-hero` | 命中/伤害各+3；独立 sustain DEX/恢复/see invis/DEX/CON/LIFE/SPEED/levitation/CHR 与 -STEALTH 分支、高抗；1/3 后再 1/A WARRIOR 激活；finalize pval<=3，见 ego.c:3519–3537。 | — |
| 110 知识之 | HELMET；0/40/1；0,0,0,3 | `knowledge` | 允许尖帽；1/7 MAGIC_MASTERY，1/A Knowledge 列表；静态 LORE2。 | LORE2 |
| 111 虔诚之 | HELMET；0/40/1；0,0,0,3 | `piety` | 尖帽拒绝；1/7 SPELL_CAP，1/A Piety 列表。 | — |
| 112 支配之 | HELMET；0/40/2；0,0,0,3 | `domination-112` | 尖帽拒绝；静态 CHR/sustain CHR/fear 抗性。 | — |
| 113 坚韧之 | HELMET；0/50/4；0,0,0,5 | `fortitude` | 尖帽拒绝；静态 CON/sustain。 | — |
| 114 狗头人的 | HELMET；10/70/4；0,5,0,2 | `the-kobold` | 尖帽拒绝；静态 STR/毒抗/-CHR。 | — |
| 115 巨魔的 | HELMET；10/60/6；0,8,5,2 | `the-troll` | 尖帽拒绝；静态 STR/恢复/-INT/光易伤。 | — |
| 116 吸血鬼的 | HELMET；40/—/16；0,0,0,3 | `the-vampire` | 尖帽拒绝；独立 1/2 冷/幽冥抗性、1/3 光易伤、1/6 Vampiric。 | DARKNESS |
| 117 阳光的 | HELMET；10/50/4；0,0,0,0 | `sunlight` | 尖帽拒绝；1/3 暗易伤，1/A Sunlight 列表。 | — |
| 118 (矮人的) | HELMET；0/—/18；0,0,12,3 | `dwarven-118` | 拒绝皮帽/龙盔/尖帽；重量 base*2/3、基础 AC+3，1/4 TUNNEL。 | — |
| 119 女武神的 | HELMET；40/—/32；5,5,0,2 | `the-valkyrie` | 尖帽拒绝；静态速度/STR/CHR/fear 抗性；固定 HEROISM。 E: `HEROISM:15:50`。 | — |
| 120 狂怒之 | HELMET；40/—/32；-10,10,-10,3 | `rage` | 尖帽拒绝；1/6 -STEALTH、1/3 混乱易伤，伤害+3+m_bonus(7,L)。 E: `BERSERK:10:50`。 | — |
| 121 托姆特的 | HELMET；10/—/2；8,-8,3,5 | `the-tomte` | 只允许 Knit Cap/Pointy Hat；重量固定 8；速度/冷抗/DEX 按 1/2 或 1/4 与等级双门，DEX 后 1/2 sustain；SPEED 触发 finalize 特例。 | — |
| 122 女巫的 | HELMET；15/—/2；0,0,0,3 | `the-witch` | 只允许 Pointy Hat；循环 1+m_bonus(5,L)，taso=min(100,L) 随获得属性递减；抵抗、装置、EASY_SPELL/DEC_MANA/SPELL_CAP/REGEN_MANA、LORE2/ESP 等分支互斥顺序不可扁平化。-WIS、光易伤和 1/A DEMON 激活只在首轮，见 ego.c:3253–3358。 | — |
| 125 心灵感应之 | CROWN；10/80/2；0,0,0,0 | `telepathy` | 冠：先 strong ESP，根据其返回值调用 weak(TRUE/FALSE)，不是固定 telepathy。 | — |
| 126 贤者的 | CROWN,HELMET；30/—/8；0,0,0,3 | `magi-headgear` | 冠或尖帽；1/3 高抗否则四次元素抗性；1/7 EASY_SPELL、1/3 -STR；1/30 SPELL_POWER/-CON，否则 1/3 伤害提升，否则仅冠 L>70 时 1/30 REGEN_MANA；L>70 时 1/10 SPEED；1/A MAGE 激活。 | XTRA_H_RES, XTRA_POWER |
| 127 力量之 | CROWN；30/—/4；0,0,0,3 | `might` | 冠：1/5 命中/伤害各 1d7，1/3 fear 抗性否则高抗；L>70 时 1/10 SPEED；1/A WARRIOR 激活。 | — |
| 128 贵族之 | CROWN；30/—/4；0,0,0,3 | `lordliness` | 冠：1/5 SPELL_CAP，两次独立 1/5 高抗，L>70 时 1/5 SPEED；1/A PRIESTLY 激活。 | XTRA_H_RES |
| 129 安格马的 | CROWN；90/—/32；10,10,0,3 | `angmar` | 冠：无独立分支；telepathy、抗性/易伤、hold life、see invis、潜行、-CON 和 TY_CURSE。 | TELEPATHY, TY_CURSE |
| 130 不信者的 | CROWN；70/—/32；0,0,0,3 | `the-unbeliever` | 冠：无独立分支；NO_MAGIC/MAGIC_RESISTANCE 与负属性待接。 | MAGIC_RESISTANCE, NO_MAGIC, XTRA_H_RES |
| 135 杀戮之 | GLOVES；0/—/2；8,8,0,0 | `slaying-135` | 1/4 进入 slaying；1+m_bonus(4,L) 次、1/8 翻倍；复用 _choose_slaying_info 权重，只加 slay，不升 kill、不加 ESP。 | AWARE |
| 136 盗贼的 | GLOVES；10/—/3；5,-5,0,4 | `the-thief` | 1/20 SPEED；共用 pval。 | — |
| 137 巨人的 | GLOVES；40/—/6；-5,10,0,4 | `the-giant` | 1/4 从音/碎片/混乱抗性中选一；1/3 混乱易伤；独立 1/2 -STEALTH/-DEX。 | — |
| 138 巫师的 | GLOVES；30/—/4；-10,-10,-20,3 | `wizard-gloves` | 1/4 从混乱/盲/光抗中选一；1/2 -STR，1/3 -CON，1/30 DEVICE_POWER；负 C 加成和 finalize pval 限幅。 | — |
| 139 伊克人的 | GLOVES；0/50/16；-10,-10,0,7 | `the-yeek` | 1/10 酸免疫；静态酸抗与负属性/潜行。 | — |
| 140 源氏的 | GLOVES；60/—/16；8,0,0,3 | `genji` | 无独立分支；Genji 双持消费者待接。 | DUAL_WIELDING |
| 141 狙击手的 | GLOVES；50/—/16；0,5,0,3 | `the-sniper` | 命中覆盖为 5+1d10，后做 C；静态射击 flag 消费者。 | XTRA_MIGHT |
| 142 狂战士的 | GLOVES；70/—/32；-15,8,-15,3 | `the-berserker` | 命中/伤害/AC 覆盖 -10/+10/-10，后做 C；pval 1d2+1/15。 E: `WHIRLWIND_ATTACK:30:250`。 | NO_ENCHANT |
| 145 漂浮之 | BOOTS；0/70/1；0,0,0,0 | `levitation` | 1/2 高抗；静态 levitation。 | AWARE, LEVITATION |
| 146 (侏儒的) | BOOTS；0/60/2；0,0,0,0 | `gnomish` | 无独立分支；FREE_ACT 和固定 PHASE_DOOR。 E: `PHASE_DOOR:1:5`。 | — |
| 147 (矮人的) | BOOTS；15/80/3；0,0,10,3 | `dwarven-147` | 只允许 metal-shod，mithril-shod 还要通过 1/2 接受；重量 base*2/3、基础 AC+4，1/4 sustain CON。 | — |
| 148 速度之 | BOOTS；30/—/5；0,0,0,0 | `speed` | pval=1+m_bonus(3+(min(90,L)-30)/10,L)，L<30 时 amt=3；不是 1d(max)。 | AWARE |
| 149 精灵的 | BOOTS；30/—/8；0,0,0,3 | `elvenkind-149` | 独立 1/2 高抗/levitation；L>70 finalize AC 和 pval 增幅。 | — |
| 150 费诺的 | BOOTS；100/—/250；0,0,0,0 | `feanor` | pval=6+m_bonus(9,L)；固定 SPEED 激活。 E: `SPEED:30:200`。 | SPEED |
| 151 妖精的 | BOOTS；30/—/6；0,0,0,3 | `the-sprite` | 1/2 高抗；静态速度/光抗/levitation。 | LEVITATION |
| 152 魔像的 | BOOTS；40/—/12；8,8,15,3 | `the-golem` | 无独立分支；负速度/-DEX、STR/CON/LIFE、毒抗/免麻/see invis 与 XTRA_H_RES。 | DEC_SPEED, XTRA_H_RES |

## 基础物品身份和获取缺口

在 k_info 中找到 94 条 tval 30–38 记录，38 条已有正式普通物品，56 条尚无对应普通物品定义（不把固定神器视为普通基础物品）。
37 条来自 selection，Fur Cloak 来自 active adaptation。下表中文直接取 kind_name_zh，不新造译名。
`已回填`只确认身份，不宣称已有物品所有原版固有属性都已验收；例如 Elven Cloak 的基础随机 pval、特殊龙类固有随机抗性仍需对应批次处理。

| source index | tval/sval | 权威中文名 | 正式 item ID / 本批状态 |
| --- | --- | --- | --- |
| 195 | 35/1 | & 件~披风 | `demo.item.cloak`；已回填 |
| 196 | 35/2 | & 件~精灵披风 | `demo.item.elven-cloak`；已回填 |
| 197 | 35/3 | & 件~毛皮披风 | `demo.item.fur-cloak`；已回填 |
| 198 | 35/5 | & 件~空灵披风 | 缺普通基础物品；后续按对应类型导入 |
| 199 | 35/6 | & 件~暗影披风 | 缺普通基础物品；后续按对应类型导入 |
| 200 | 35/7 | & 对~龙翼 | 缺普通基础物品；后续按对应类型导入 |
| 205 | 30/2 | & 双~软皮靴 | `demo.item.soft-leather-boots`；已回填 |
| 206 | 30/3 | & 双~硬皮靴 | `demo.item.pair-of-hard-leather-boots`；已回填 |
| 207 | 30/4 | & 双~龙靴 | 缺普通基础物品；后续按对应类型导入 |
| 208 | 30/5 | & 双~铁头靴 | `demo.item.pair-of-metal-shod-boots`；已回填 |
| 209 | 30/6 | & 双~秘银铁头靴 | 缺普通基础物品；后续按对应类型导入 |
| 212 | 32/2 | & 顶~硬皮帽 | `demo.item.hard-leather-cap`；已回填 |
| 213 | 32/3 | & 顶~金属帽 | `demo.item.metal-cap`；已回填 |
| 214 | 32/4 | & 顶~阵笠 | `demo.item.jingasa`；已回填 |
| 215 | 32/5 | & 顶~铁盔 | `demo.item.iron-helm`；已回填 |
| 216 | 32/6 | & 顶~钢盔 | 缺普通基础物品；后续按对应类型导入 |
| 217 | 32/7 | & 顶~秘银头盔 | 缺普通基础物品；后续按对应类型导入 |
| 218 | 32/8 | & 顶~龙盔 | 缺普通基础物品；后续按对应类型导入 |
| 219 | 32/9 | & 顶~兜 | 缺普通基础物品；后续按对应类型导入 |
| 220 | 33/10 | & 顶~铁王冠 | 缺普通基础物品；后续按对应类型导入 |
| 221 | 33/11 | & 顶~金王冠 | 缺普通基础物品；后续按对应类型导入 |
| 222 | 33/12 | & 顶~镶钻王冠 | 缺普通基础物品；后续按对应类型导入 |
| 223 | 33/50 | & 顶~重铁王冠 | 缺普通基础物品；后续按对应类型导入 |
| 224 | 32/1 | & 顶~针织帽 | `demo.item.knit-cap`；已回填 |
| 225 | 32/10 | & 顶~尖帽子 | `demo.item.pointy-hat`；已回填 |
| 227 | 31/1 | & 副~皮手套 | `demo.item.leather-gloves`；已回填 |
| 228 | 31/2 | & 副~镶钉皮手套 | `demo.item.set-of-studded-leather-gloves`；已回填 |
| 229 | 31/3 | & 副~铁护手 | `demo.item.set-of-gauntlets`；已回填 |
| 230 | 31/4 | & 副~带刺铁护手 | `demo.item.set-of-spiked-gauntlets`；已回填 |
| 231 | 31/5 | & 副~秘银护手 | 缺普通基础物品；后续按对应类型导入 |
| 232 | 31/6 | & 副~龙皮手套 | 缺普通基础物品；后续按对应类型导入 |
| 233 | 31/7 | & 副~搏击拳套 | 缺普通基础物品；后续按对应类型导入 |
| 234 | 31/8 | & 只~手 | 缺普通基础物品；后续按对应类型导入 |
| 235 | 34/2 | & 面~小皮盾 | `demo.item.small-leather-shield`；已回填 |
| 236 | 34/3 | & 面~小金属盾 | `demo.item.small-metal-shield`；已回填 |
| 237 | 34/4 | & 面~大皮盾 | `demo.item.large-leather-shield`；已回填 |
| 238 | 34/5 | & 面~大金属盾 | `demo.item.large-metal-shield`；已回填 |
| 239 | 34/6 | & 面~龙皮盾 | 缺普通基础物品；后续按对应类型导入 |
| 240 | 34/7 | & 面~骑士盾 | 缺普通基础物品；后续按对应类型导入 |
| 241 | 34/8 | & 面~秘银盾 | 缺普通基础物品；后续按对应类型导入 |
| 242 | 34/10 | & 面~镜之盾 | `demo.item.mirror-shield`；已回填 |
| 243 | 34/50 | & 面~镜子 | 缺普通基础物品；后续按对应类型导入 |
| 245 | 36/0 | & 件~T恤 | 缺普通基础物品；后续按对应类型导入 |
| 246 | 36/1 | & 件~肮脏的破布 | `demo.item.filthy-rag`；已回填 |
| 247 | 36/2 | & 件~长袍 | `demo.item.robe`；已回填 |
| 248 | 36/3 | 纸装甲~ | `demo.item.paper-armour`；已回填 |
| 249 | 36/4 | 软皮甲~ | `demo.item.soft-leather-armour`；已回填 |
| 250 | 36/5 | 软镶钉皮甲~ | `demo.item.soft-studded-leather`；已回填 |
| 251 | 36/6 | 硬皮甲~ | `demo.item.hard-leather-armour`；已回填 |
| 252 | 36/7 | 硬镶钉皮甲~ | `demo.item.hard-studded-leather`；已回填 |
| 253 | 36/8 | 犀牛皮甲~ | `demo.item.rhino-hide-armour`；已回填 |
| 254 | 36/9 | 绳甲~ | `demo.item.cord-armour`；已回填 |
| 255 | 36/10 | 内衬甲~ | `demo.item.padded-armour`；已回填 |
| 256 | 36/11 | 皮鳞甲~ | `demo.item.leather-scale-mail`；已回填 |
| 257 | 36/12 | & 件~皮夹克 | `demo.item.leather-jacket`；已回填 |
| 258 | 36/13 | 黑衣 | 缺普通基础物品；后续按对应类型导入 |
| 259 | 36/15 | 石皮甲~ | 缺普通基础物品；后续按对应类型导入 |
| 260 | 36/50 | 性感泳装~ | 缺普通基础物品；后续按对应类型导入 |
| 261 | 36/60 | & 件~长袍 | 缺普通基础物品；后续按对应类型导入 |
| 262 | 36/63 | 戏服~ | 缺普通基础物品；后续按对应类型导入 |
| 270 | 37/1 | 生锈的链甲~ | 缺普通基础物品；后续按对应类型导入 |
| 271 | 37/2 | 环甲~ | `demo.item.ring-mail`；已回填 |
| 272 | 37/3 | 金属鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 273 | 37/4 | 链甲~ | `demo.item.chain-mail`；已回填 |
| 274 | 37/5 | 双层环甲~ | 缺普通基础物品；后续按对应类型导入 |
| 275 | 37/6 | 强化链甲~ | `demo.item.augmented-chain-mail`；已回填 |
| 276 | 37/7 | 双层链甲~ | 缺普通基础物品；后续按对应类型导入 |
| 277 | 37/8 | 条板链甲~ | 缺普通基础物品；后续按对应类型导入 |
| 278 | 37/9 | 金属布面甲~ | 缺普通基础物品；后续按对应类型导入 |
| 279 | 37/10 | 板环甲~ | 缺普通基础物品；后续按对应类型导入 |
| 280 | 37/11 | 胴丸~ | 缺普通基础物品；后续按对应类型导入 |
| 281 | 37/12 | 半身板甲~ | 缺普通基础物品；后续按对应类型导入 |
| 282 | 37/13 | 金属扎甲~ | `demo.item.metal-lamellar-armour`；已回填 |
| 283 | 37/14 | 腹卷~ | 缺普通基础物品；后续按对应类型导入 |
| 284 | 37/15 | 全身板甲~ | 缺普通基础物品；后续按对应类型导入 |
| 285 | 37/16 | 大铠~ | 缺普通基础物品；后续按对应类型导入 |
| 286 | 37/18 | 罗纹板甲~ | 缺普通基础物品；后续按对应类型导入 |
| 287 | 37/20 | 秘银链甲~ | 缺普通基础物品；后续按对应类型导入 |
| 288 | 37/25 | 秘银板甲~ | 缺普通基础物品；后续按对应类型导入 |
| 289 | 37/30 | 精金板甲~ | 缺普通基础物品；后续按对应类型导入 |
| 292 | 38/1 | 黑龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 293 | 38/2 | 蓝龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 294 | 38/3 | 白龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 295 | 38/4 | 红龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 296 | 38/5 | 绿龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 297 | 38/6 | 多彩龙鳞甲~ | `demo.item.multi-hued-dragon-scale-mail`；已回填 |
| 298 | 38/10 | 伪龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 299 | 38/12 | 律法龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 300 | 38/14 | 青铜龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 301 | 38/15 | 银龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 302 | 38/16 | 金龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 303 | 38/18 | 混沌龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 304 | 38/20 | 平衡龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |
| 305 | 38/30 | 力量龙鳞甲~ | 缺普通基础物品；后续按对应类型导入 |

优先依赖：盾批次核对龙盾/镜盾排除（镜盾已存在）；头冠当前没有普通基础 crown，开放 CROWN 前须先补；
Twilight 依赖 Yoi yami Robe；龙甲 Crusade 依赖 Gold/Law，Death 依赖 Chaos；
Mithril-shod Boots 的 1/2 拒绝分支需补对应基础物品后验收。Wizardstaff 不属于本 94 条护甲清单，随 DEC_MANA 底座另交付。

### 龙甲基础激活

下列 E: 仍是基础物品契约，不是 ego 显式 E:。Breath 必须从最终基础 kind 复制；Lore/Death 覆盖它。

| kind | 权威中文名 | 原版 E: | 现状 |
| --- | --- | --- | --- |
| 292 (38/1) | 黑龙鳞甲~ | BREATHE_ACID:30:30:150; 你喷吐出酸液。 | 基础物品及激活待导入 |
| 293 (38/2) | 蓝龙鳞甲~ | BREATHE_ELEC:30:30:150; 你喷吐出闪电。 | 基础物品及激活待导入 |
| 294 (38/3) | 白龙鳞甲~ | BREATHE_COLD:30:30:150; 你喷吐出冰霜。 | 基础物品及激活待导入 |
| 295 (38/4) | 红龙鳞甲~ | BREATHE_FIRE:30:50:150; 你喷吐出火焰。 | 基础物品及激活待导入 |
| 296 (38/5) | 绿龙鳞甲~ | BREATHE_POIS:30:40:150; 你喷吐出毒气。 | 基础物品及激活待导入 |
| 297 (38/6) | 多彩龙鳞甲~ | BREATHE_ONE_MULTIHUED:40:70:250 | 已存在，需 Ego 替换/缩放 |
| 298 (38/10) | 伪龙鳞甲~ | BREATHE_ONE_SHINING:40:50:200 | 基础物品及激活待导入 |
| 299 (38/12) | 律法龙鳞甲~ | BREATHE_ONE_LAW:50:100:230 | 基础物品及激活待导入 |
| 300 (38/14) | 青铜龙鳞甲~ | BREATHE_CONF:30:40:120; 你喷吐出混乱。 | 基础物品及激活待导入 |
| 301 (38/15) | 银龙鳞甲~ | BREATHE_INERTIA:30:40:130; 你喷吐出惰性。 | 基础物品及激活待导入 |
| 302 (38/16) | 金龙鳞甲~ | BREATHE_SOUND:30:40:130; 你喷吐出声波。 | 基础物品及激活待导入 |
| 303 (38/18) | 混沌龙鳞甲~ | BREATHE_ONE_CHAOS:50:100:220 | 基础物品及激活待导入 |
| 304 (38/20) | 平衡龙鳞甲~ | BREATHE_ONE_BALANCE:50:100:250 | 基础物品及激活待导入 |
| 305 (38/30) | 力量龙鳞甲~ | BREATHE_ELEMENTS:50:100:300; 你喷吐出元素！ | 基础物品及激活待导入 |

## 随机激活候选审计

以下列表以 token 标识规则，不作为玩家中文显示名。激活中文显示名仍必须从权威运行时表/源字符串取值，
对应实现批次核对并生成；本批不本地翻译。基础 effect 存在也需验证目标、power、范围、冷却和学习行为。

| 调用方 | 显式 choices（按 devices 表加权，不按本列表顺序等概率） |
| --- | --- |
| Lore 85 | IDENTIFY_FULL, DETECT_ALL, ENLIGHTENMENT, CLAIRVOYANCE, SELF_KNOWLEDGE |
| Death 92 | GENOCIDE, MASS_GENOCIDE, WRAITHFORM, DARKNESS_STORM |
| Sunlight 117 | LITE_AREA, LITE_MAP_AREA, BOLT_LITE, BEAM_LITE_WEAK, BEAM_LITE, BALL_LITE, BREATHE_LITE, CONFUSING_LITE |
| Knowledge 110 | IDENTIFY, IDENTIFY_FULL, PROBING, DETECT_TRAPS, DETECT_MONSTERS, DETECT_OBJECTS, DETECT_ALL, ENLIGHTENMENT, CLAIRVOYANCE, SELF_KNOWLEDGE |
| Piety 111 | HEAL, CURING, RESTORE_STATS, RESTORE_EXP, HEAL_CURING, CURE_POIS, CURE_FEAR, REMOVE_CURSE, REMOVE_ALL_CURSE, CLARITY |

下表列出实际 bias 候选的 `token(level/cost/rarity)`；运行时再按 `object_level/3` 下限过滤，
选择后仍保留源表顺序。同一候选可以属于多个 bias。ELEMENTAL 是组合位，不能把它当不存在的单独标签。

| bias / 护甲调用方 | 候选 |
| --- | --- |
| ELEMENTAL / 51 | RESIST_ACID(15/100/1), RESIST_ELEC(15/100/1), RESIST_FIRE(15/100/1), RESIST_COLD(15/100/1), RESIST_POIS(30/150/2), CURE_POIS(10/50/1), BOLT_ACID(15/20/1), BOLT_ELEC(15/20/1), BOLT_FIRE(15/20/1), BOLT_COLD(15/20/1), BOLT_POIS(10/10/1), BOLT_ICE(50/100/4), BOLT_PLASMA(50/100/4), BEAM_ACID(20/20/2), BEAM_ELEC(20/20/2), BEAM_FIRE(20/20/2), BEAM_COLD(20/20/2), BALL_ACID(25/50/1), BALL_ELEC(25/50/1), BALL_FIRE(25/50/1), BALL_COLD(25/50/1), BALL_POIS(10/5/1), BREATHE_ACID(40/100/2), BREATHE_ELEC(40/100/2), BREATHE_FIRE(40/100/2), BREATHE_COLD(40/100/2), BREATHE_POIS(40/100/2) |
| WARRIOR / 70,104,127 | WHIRLWIND_ATTACK(50/100/4), STONE_SKIN(25/150/2), HEROISM(15/100/1), BERSERK(20/100/2), SPEED_HERO(35/200/6) |
| DEMON / 73,74,75,122 | DESTRUCTION(50/250/6), RECHARGE_FROM_DEVICE(35/500/3), RECHARGE_FROM_PLAYER(90/500/16), RESIST_FIRE(15/100/1), BLESS(10/100/1), HEROISM(15/100/1), SUMMON_DEMON(66/666/2), CHARM_DEMON(50/500/1), BOLT_FIRE(15/20/1), BOLT_NETHER(20/25/1), BEAM_FIRE(20/20/2), BALL_FIRE(25/50/1), BALL_NETHER(40/50/2), BREATHE_NETHER(50/75/2), ROCKET(70/200/8), AGGRAVATE(10/100/1) |
| MAGE / 126 | LITE_AREA(1/10/1), CLAIRVOYANCE(35/100/8), DETECT_MONSTERS(5/20/1), PHASE_DOOR(8/20/1), TELEPORT_AWAY(20/50/2), DIMENSION_DOOR(50/100/8), RECALL(25/100/2), RECHARGE_FROM_DEVICE(35/500/3), RECHARGE_FROM_PLAYER(90/500/16), IDENTIFY(15/50/1), IDENTIFY_FULL(50/200/3), PROBING(30/50/1), RUNE_EXPLOSIVE(30/100/2), BANISH_ALL(50/100/8), TELEKINESIS(25/100/2), ALCHEMY(70/100/4), SELF_KNOWLEDGE(70/500/3), SPEED(25/150/4), TELEPATHY(30/150/8), INVULNERABILITY(90/777/16), SUMMON_MONSTERS(30/500/1), SUMMON_PHANTASMAL(25/150/1), SUMMON_ELEMENTAL(30/150/1), SUMMON_DRAGON(60/600/2), CHARM_MONSTER(30/100/1), RESTORE_MANA(80/900/8), CLARITY(20/15/12), GREAT_CLARITY(80/75/64), BOLT_MISSILE(1/10/1), BOLT_WATER(55/150/4), BOLT_MANA(50/100/4), BOLT_ICE(50/100/4), BALL_WATER(70/200/4), BALL_MANA(80/200/6), BREATHE_WATER(65/150/8), MANA_STORM(80/250/8), STASIS_MONSTERS(50/250/8) |
| PRIESTLY / 128 | ENLIGHTENMENT(20/50/2), DETECT_EVIL(20/30/1), RUNE_PROTECTION(70/500/4), BANISH_EVIL(50/100/4), PROT_EVIL(35/200/2), HOLY_GRAIL(50/500/4), BLESS(10/100/1), HEROISM(15/100/1), SUMMON_ANGEL(70/777/8), RESTORE_STATS(50/600/2), RESTORE_EXP(40/500/1), RESTORING(70/800/3), HEAL(40/500/1), CURING(45/200/1), HEAL_CURING(60/900/4), HEAL_CURING_HERO(70/900/4), CURE_POIS(10/50/1), CURE_FEAR(25/100/1), CURE_FEAR_POIS(30/100/1), REMOVE_CURSE(30/200/1), REMOVE_ALL_CURSE(70/500/4), CLARITY(20/15/12), GREAT_CLARITY(80/75/64), DISPEL_EVIL(50/200/2), DISPEL_EVIL_HERO(60/250/3), DISPEL_UNDEAD(60/200/2), HOLINESS(50/250/8) |
| NECROMANTIC / 103 | GENOCIDE(70/500/16), MASS_GENOCIDE(80/750/16), GENOCIDE_ONE(60/250/8), WRAITHFORM(90/666/16), SUMMON_UNDEAD(60/600/2), CHARM_UNDEAD(50/500/1), BOLT_DARK(20/25/1), BOLT_NETHER(20/25/1), BALL_DARK(66/100/2), BALL_NETHER(40/50/2), BREATHE_DARK(50/125/3), BREATHE_NETHER(50/75/2), DISPEL_GOOD(50/150/1), DISPEL_LIFE(55/200/1), DRAIN_LIFE(40/100/2), DARKNESS_STORM(70/500/32), ANIMATE_DEAD(25/100/1), SCARE_MONSTERS(20/100/1) |
| FIRE / 96 | RESIST_FIRE(15/100/1), BOLT_FIRE(15/20/1), BOLT_PLASMA(50/100/4), BEAM_FIRE(20/20/2), BALL_FIRE(25/50/1), BREATHE_FIRE(40/100/2) |
| ELEC / 97 | RESIST_ELEC(15/100/1), BOLT_ELEC(15/20/1), BEAM_ELEC(20/20/2), BALL_ELEC(25/50/1), BREATHE_ELEC(40/100/2) |
| COLD / 98 | RESIST_COLD(15/100/1), BOLT_COLD(15/20/1), BOLT_ICE(50/100/4), BEAM_COLD(20/20/2), BALL_COLD(25/50/1), BREATHE_COLD(40/100/2) |

上述随机候选并集为 114 个 token；当前 `legacy_device_item_effect` 对其中 105 个已有映射分支。
剩余 9 个在该 helper 返回 None（不等于整个游戏没有相近效果）：

| token | 护甲调用方 / 具体缺口 |
| --- | --- |
| BERSERK | WARRIOR、固定 72/120；复用已有 apply-berserk-strength，补生成 profile 与正确源 power/cost |
| WHIRLWIND_ATTACK | WARRIOR、固定 142；需要完整近邻攻击执行和 profile，不能用命中单个目标替代 |
| STONE_SKIN | WARRIOR；现有 resolve_item_stone_skin 可复用，补 helper/profile |
| SPEED_HERO | WARRIOR；组合 haste/heroism，保留原版持续时间和 RNG |
| BOLT_MISSILE | MAGE 的低等级候选；补原版伤害曲线、目标和 helper |
| DETECT_MONSTERS | MAGE 与 Knowledge 列表；复用探测流程，补原版范围/知识与 helper |
| IDENTIFY | MAGE 与 Knowledge 列表；复用物品选择/鉴定流程，补 helper |
| LITE_AREA | MAGE 与 Sunlight 列表；补原版照明与伤害组合的 profile |
| PHASE_DOOR | MAGE、固定 146；复用短距传送，补 helper 和固定激活参数 |

`RfbActivationBiasDefinition` 目前没有 Warrior，importer 的 bias 映射也没有该项。
已有固定 helper 只接受 source 11/15/24/42，护甲的 7 条显式 E:（72/95/119/120/142/146/150）
均须新增分派；不能因 token 与已有武器相同而漏掉 source-index 限制。显式列表还需要不带 bias 等级
过滤的加权选择入口。源 cost 到 tick 的换算沿用现有 *10 规则；龙甲缩放在实例上进行。

消费者开放门槛：复用现有 E3 激活实例和 effect program，逐 token 核对原版 `devices.c` 的目标模式、
失败、费用、恢复与鉴定，再开放对应类型。E5.0 的 105 个映射计数不代表新增护甲激活已经验收。

## 复现与本批验证

```powershell
cargo run -q -p rfb-legacy-import -- audit-egos D:/codex/Frogcomposband/master
$env:RFB_LEGACY_SOURCE = 'D:/codex/Frogcomposband/master'
cargo run -q -p rfb-legacy-import -- sync-demo-armor-ego-identities packs/rfb-demo-original
cargo test -p rfb-legacy-import armor_ --lib
cargo test -p rfb-core armor_base_identities --lib
cargo test -p rfb-core base_item_natural_egos --lib
cargo run -q -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -q -p rfb-contract -- verify-category tests/fixtures/active/baseline-policy.json equipment inventory tasks town
```

回填命令先验证全部源记录、selection/adaptation 和正式文件身份再开始写文件；第二次执行结果相同。
聚焦测试覆盖 76 条静态记录漂移、中文缺失、显式激活变化、5 个已有 ID 和 38 件基础身份；
核心测试验证全部 38 件护甲在等级 1/30/80/100 不进入 RFB Ego 分派且消耗零 RNG。
原有自然生成命名种子测试和 11 个 fixture 通过，未刷新 fixture。没有运行桌面或全量回放验收。

动态 C 分支和消费者是本次人工源码审计结果，静态 JSON 契约不会自动检测 ego.c 的任意代码变化；
各 E5 实现批次必须重新读取当时的 master 对应分支，增加真实物化/消费者测试。
