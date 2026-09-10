# 法师来源与消费者审计

初审日期：2026-09-10，对应[法师计划](mage-class-plan.md)第一步，代码基线 `a09a334df`。以下源→实现差异表记录初审基线；截至 2026-09-11 已完成前四步，见文末当前进度，**普通创角入口尚未开放**。工作树既存 `release/` 保留。

RFB 来源为 `D:/codex/Frogcomposband/master` 的 `master` Git 对象，实际提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，第四步复核未变。以下源路径和行号均指此提交，通过 `git show` / `git grep` 读取；实现路径指本项目。内容保持 1.413.0；当前协议 1.251、State Hash Schema 124、save header/payload 为 14/19。

## 范围和审计方法

全读 `src/mage.c`、`lib/edit/m_info.txt N:1`、`lib/edit/s_info.txt N:1`，追踪学习、改换、施法、出生、属性、美德、装备感知、装置、生成、任务、公会和保存。除 `CLASS_MAGE` 外，另查 `realm1/realm2`、caster options、class flags、职业表索引及当前 Build 领域消费者，避免漏掉没有显式 Mage 分支的规则。

本轮八领域为 `life/sorcery/nature/death/arcane/daemon/crusade/armageddon`，中文名依次是生命、咒术、自然、死亡、奥秘、恶魔、圣战、毁灭。源 `tables.c:1714,1788,1863` 的两组候选各有 12 项；四个未接入领域 `chaos/trump/craft/law` 保留后续范围。`law` 使用技术法术表，不能因为 `m_info N:1` 没有该区块就判为非法领域。

八领域形成 56 个有序、互异组合；Mage 不受 Priest 的善恶互斥限制。每领域按现有四本书的 rank、书内次序对齐 32 项源法术，共核对 256 项职业参数与现有高阶法师覆盖。这里核对的是职业参数及身份相关效果调用链，不等于重新逐项验收全部公共法术效果。公共效果沿用其既有测试；本审计发现的可达差异必须补齐后才能开放 Mage。

## 1. 出生、成长和装备

| 源规则 | 当前实现与差异 | 落地步骤 |
| --- | --- | --- |
| `mage.c`：STR/INT/WIS/DEX/CON/CHR 为 −4/+3/0/+1/−2/−2；life 95，base HP 0，exp 130，pets 30 | [Class 定义](../packs/rfb-demo-original/classes/)没有 Mage；不能复制 High-Mage 的 INT +4、DEX 0、life 94、pets 25 | 第二步新增共享 Class、actor、skill set 和 56 个薄 Build |
| 基础技能 dis/dev/sav/stl/srh/fos/thn/thb 为 30/40/38/3/16/20/34/20，每十级成长 7/15/11/0/0/0/6/7；`combat.c:181` blows 为 400/100/20 | 现有技能与攻击参数可表达；与 High-Mage 分开导入 | 第二步 |
| `s_info N:1` 全部 320 个 W 与三个 S 记录；`skills.c` 出生把默认 Unskilled 起点提高至 2000，但不超过武器上限 | 默认武器 2000→4000；匕首、短杖、巫师法杖 4000→8000；投石索 4000→6000；短刀 2000→6000；龙牙和毒针 2000→8000。双节棍、钓竿为 0→0；两个 tval 的 sval 63 为 2000→8000。不能只复制已经存在的六个 override 而忽略未导入身份 | 第二步导入当前物品映射；未导入源种类保留来源范围，不伪造 item ID |
| `s_info` martial 0→4000，dual/riding 0→0 | 复用现有 proficiency 字段与战斗调用者；0 上限不等于禁止装备或骑乘入口 | 第二步 |
| `py_birth.c:193` 出生匕首、长袍、主副各第一本书；Mage 无 High-Mage 的额外清晰药水/魔杖出生赠品 | [出生 Build](../packs/rfb-demo-original/builds/)、种族公共出生可复用；书来自各自领域，不能只给主领域书或自动学会两个法术 | 第二步 |
| `py_birth.c` 属性建议为 16/17/9/9/16/9；`xtra1.c:_calc_xtra_hp_aux` Mage 使用纯三次权重 0/0/1 | 当前公共出生属性与 HP progression 不复刻源点购、HP 掷点和额外 HP 曲线；[progression.rs](../crates/rfb-core/src/game/progression.rs)按已有 progression、base HP、life 与 CON 计算。**保留公共适配**，不能把 1–50 级检查描述为完整复刻源 HP 曲线 | 第二/七步明确验证当前成长和源职业修正 |
| `races_a.c:1222` 龙人变形攻击有效等级，Mage 与 High-Mage 同为调整后的 80%；未发现 Mage 特有冬贝利禁配 | [player_stats.rs](../crates/rfb-core/src/game/player_stats.rs)目前仅 High-Mage 走 80，新增 Mage 会误走默认 100；决斗者的禁配不得照搬 | 第二步补分支及种族交叉 |
| `virtue.c` Mage 固有 Knowledge、Enchantment，随后按出生种族/领域补充 | [virtues.rs](../crates/rfb-core/src/game/virtues.rs)需增加职业身份。领域美德是出生初始化输入；改换副领域不重抽、清空或补发美德 | 第二步职业；第四步保持出生语义 |
| `mage.c` SENSE1 MED/WEAK、SENSE2 FAST/STRONG；`dungeon.c:183–290` 两个计时分别 20000、9000，经 WIS/Knowledge 调整后除以 `(L+10)^2+40`，混乱阻止；Knowledge≥100 可强化第一类 | [item_knowledge.rs](../crates/rfb-core/src/game/item_knowledge.rs)的周期流程目前限 Mindcrafter；不能用狂战士/决斗者的即时感知替代。复用两个物品集合及公共感知，接 Mage 的频率和强弱 | 第二步，连同实际回合调用 |

`s_info` 中尚不存在正式 item 的槽位保留数值来源即可；不能为了填矩阵导入无关武器。上限换算用源 proficiency 常量及出生修正，不将 W 的 0/1/2/4 当实际熟练度。

## 2. 法力、职业参数与可达效果差异

| 源规则 | 当前实现与缺口 | 落地步骤 |
| --- | --- | --- |
| `mage.c` INT 施法，负重 430/100/600，DEC_MANA、GLOVE_ENCUMBRANCE；普通 Mage 没有专修领域额外伤害和被动 | [player_abilities.rs](../crates/rfb-core/src/game/player_abilities.rs)已有 RfbMana、负重及装备能力；Mage capacityPercent 应为 100，不能继承 115 或 High-Mage 的 spellDamageBonus。领域存在不等于获得 High-Mage 专修被动 | 第二步 |
| `xtra1.c` 法力基值 `adj_mag_mana × (L+3)/4`，非零加 1，再受真种族 INT 等修正；手套无 FREE_ACT/MAGIC_MASTERY/正 DEX 则法力 75%；超重按 600 分母扣减 | 复用现有 RfbMana 和 caster encumbrance；出生、升级及换装检查真实最大 MP。公共资源刷新一般 clamp，属性变化路径另按比例缩放；源 `calc_mana` 用百分比整数比例重算当前 MP | 第二步保留公共资源刷新适配并验证；不顺带改所有职业资源语义 |
| `CLASS_REGEN_MANA` 经 `xtra1.c:4632` 设置 mana_regen，`dungeon.c:1601` 正向 upkeep regen ×2 | 现有 `resourceRecoveryPercent:200` 可复用；不能只添加一个无消费者的 flag | 第二步 |
| `m_info N:1 T` 每项是 minimum level / mana / fail / sexp | 256 项与 High-Mage 对应三元组均不相同，必须使用 Mage 表；现有 realm abilityOverride 没有 first-success XP 覆盖字段 | 第二步新增必要内容表达，第三步消费奖励 |
| `do-spell.c:202` beam 概率 Mage 为 L，High-Mage 为 L+10 | 复用 beam 参数按 Mage 配置；不能用专修加成作近似 | 第二步 |
| `do-spell.c:3609` 死亡 entropy orb、`:6041` 恶魔第 7 项 hellish flame，Mage 属较强职业组：基础伤害 L+L/2，半径 L<30 为 2、否则 3 | 两个 program 的基值是 3d6/radius 2，但 [death-entropy-orb ability](../packs/rfb-demo-original/abilities/death-entropy-orb.json)和 [daemon-hellish-flame ability](../packs/rfb-demo-original/abilities/daemon-hellish-flame.json)已含 damage-bonus 3L/2 与 radius L/30 的缩放。第二步沿完整装配链复核，纠正第一步只读 program 时的缺口误判；复用即可。恶魔这里不是高阶 `daemon-hellfire` | 第二步验证 Mage 无专修伤害、30 级实际投影 |
| `lawyer.c:22` Life index 23 `life-warding-true` 在第一领域非 Life 时最低等级 99；`spells3.c:1867` 普通 glyph 总量 11，主 Life 豁免 | 当前 [life-warding-true](../packs/rfb-demo-original/abilityPrograms/life-warding-true.json)仅创建当前/邻接 glyph，静态等级覆盖无法体现主副资格；必须实现主领域限制并核对地形生成限额，不能让副 Life 在 50 级学会此项 | 第二步资格，第三步学习/施法 |
| `lawyer.c:13` Death index 21 `death-vampirism-true` 费用加 `clamp(base,50,100)`，总量封顶 250；Mage 基础 35→85 | 当前静态绑定/High-Mage 覆盖无此钩子；不能只从 m_info 导入 35。修正发生在熟练度/装备减耗之前 | 第二/三步 |
| `spells3.c:3238` 熟练度减耗与 DEC_MANA 合并整数除法：`max(1, ((base*(3800-prof)+2399)*factor)/(2400*4))`，factor=3 或 4 | 当前先向上取整再乘 3/4，存在差异，例如 base=3、prof=0、有减耗，源为 4、当前为 3 | 第三步修公共算式，覆盖既有调用者 |
| `spells3.c:3397` Mage 副领域失败率 +5，在 min fail 与后续修正之前；随后涉及装备、virtue、stun、95 封顶、Expert/Master 各减 1 等顺序 | 当前失败率没有主副参数；需按源顺序接入，不在最终结果上简单 +5。装备 easy realm 来自实际 realm stone，不是 Mage 天生能力 | 第三步；未导入的 realm stone 不凭空赋予 |

按书次序匹配后的参数审计结果如下。最后一列比较当前 player binding 原始 firstSuccessExperience 与 Mage 的 `minimumLevel × sexp`，不包含动态等级修正；它是导入风险证据，不是行为测试数量。

| 领域 | Mage 条目 | 与 High-Mage 等级/费用/失败三元组不同 | 原始首用 XP 不同 |
| --- | ---: | ---: | ---: |
| life | 32 | 32 | 31 |
| sorcery | 32 | 32 | 30 |
| nature | 32 | 32 | 31 |
| death | 32 | 32 | 32 |
| arcane | 32 | 32 | 27 |
| daemon | 32 | 32 | 31 |
| crusade | 32 | 32 | 30 |
| armageddon | 32 | 32 | 31 |
| 合计 | 256 | 256 | 243 |

复用现有 `m_info` 解析入口和 [abilityBooks](../packs/rfb-demo-original/abilityBooks/)稳定映射，参数留在 Class/realm profile，不复制 256 个法术效果。`do-spell.c` 的 Chaos Mana Burst 也有 Mage 分支，但该领域不在本轮；不得以此扩大到缺失四领域。

## 3. 学习、施法和熟练度

来源为 `cmd5.c:548–824,1093–1571`、`xtra1.c:calc_spells`、`spells3.c:spell_chance`；当前主要消费者为 [player_abilities.rs](../crates/rfb-core/src/game/player_abilities.rs)、[casting.rs](../crates/rfb-core/src/game/abilities/casting.rs)、[Game 命令分派](../crates/rfb-core/src/game/mod.rs)。

| 规则 | 源行为 | 当前缺口 / 第三步要求 |
| --- | --- | --- |
| 总学习机会 | Mage 等级从 1 开始，基础 `floor(adj_mag_study(INT)×L/2)`，上限为 96+4；再计额外学习机会、已花机会和遗忘数 | 当前 RfbSingleRealm 另减半、上限 32+4，并以已知集合长度判预算。新增双领域公式及不可派生的已花机会；不把两个 realm bonus 重复累加 |
| 初次学习 | 选择法术，费用为一个学习机会、100 源能量；Mage 用 Knowledge +1（即使学 Life） | 复用书本/等级/状态验证，明确成功学习的行动消耗；取消、不足和非法输入不得扣学习预算 |
| 重复研习 | 同样支付一个机会；主上限 1600、副 1400。原 prof≥1400 主→1600；≥1200 主+200、副→1400；≥900→1200+(old−900)×2/3；否则→900+old/3 | 当前拒绝 already-known，不能靠 cast_count 替代已花学习机会。只保留唯一首次学习顺序，重复研习不添加重复 ID |
| 遗忘/恢复 | 等级不足先按逆学习顺序遗忘，再处理预算不足；可恢复时从最早顺序恢复。已支付学习次数仍保留 | 当前 memory 由顺序和容量派生，显式 forget 会删顺序；新流程须区分暂时遗忘与副领域清除，不退款、不丢首次顺序 |
| 成功练习 | `dlvl=max(base_level,dun_level)`，比率 `(17+法术难度)×100/(10+dlvl)`；60/100/200/300 对应上限 1600/1200/900/0，区间插值。仅 dlvl>0、当前 prof 低于练习上限时成长；源允许有 base_level 的荒野，城镇不能练习 | 当前固定成功 +128 且在效果前登记；要在结果已知后判断有用攻击和真实环境，不能只读取地下层数 |
| 练习增量 | prof 0/200/400/600/800/1000/1200/1400/1600 的正常插值节点为 128/64/32/16/8/4/2/1/1；非攻击 ×3；增量先限制到环境上限，再受主副硬上限约束 | 复用 AbilityProgress；无效攻击、城镇、达到环境上限不能刷成长。本轮用源普通模式；Coffee 独立倍率不接入 |
| 首次有效施放 | worked 位驱动 `sexp × 调整后 minimumLevel` XP，以及源领域美德效果 | 当前以 cast_count==0 和静态 binding XP 发奖；需保留与成功/失败计数语义一致的首次使用状态，改换副领域后正确清除 |
| 失败 | 扣 MP、花行动；Death 另有 rank/index 相关反噬：概率 `randint1(100)<index`，最高书另可能 sanity 分支，否则书阶 d6 HP，index>15 另有 1/6 经验损失且受 hold life 保护 | 当前仅有 Mindcraft 专项失败处理，未覆盖此路径；补 Death 当前可达反噬及已有精神/经验系统映射，不静默丢弃 |
| 施放取消 | 源先扣 MP、掷失败，再进入部分效果目标选择；成功分支取消返 MP/能量，但可能已经消耗 RNG。MP 不足直接拒绝，没有透支施法 | 本项目保留公共“先校验完整目标再执行”的命令边界，无效/取消目标不提前掷失败；这是明确适配。改换副领域的取消另按下节处理，不能套用同一回滚 |

源 save 的 worked 是布尔状态，当前 cast_count 是计数；能用计数准确表达时不另造重复状态。需要保存的是后续行为真正依赖的事实，不能为复刻 C 的位布局添加冗余字段。

## 4. 第二领域改换与消费者

`cmd5.c:573 change_realm2` 的顺序是：开始学习时已有可用学习机会 → 选择背包或脚下的第三领域书 → 确认改换 → 清理副领域状态并更换领域 → 再选具体法术。清理包括第二领域 32 项熟练度、learned/worked/forgotten 位和学习顺序；保留主领域顺序，旧副领域记入 old_realm。**不退还 learned_spells**。

拒绝确认不改变领域；确认后取消选择法术，已经改换的领域仍然保留，本次不支付学习机会和行动能量。源确认后调用 `autopick_alter_obj(book, FALSE)`：可自动鉴定、题字，不拾取或销毁；鉴定可以消耗真实装置 SP 或卷轴，不能误算为学习付款。第四步复用现有 Mogaminator，并排除确认后的二次背包销毁处理。改换本身不掷随机数。源 old_realm 供角色报告展示，不是禁止换回的黑名单。换回必须重新学习和成长，不能恢复被清除的副领域进度。

Build 保留出生身份，主领域不变；当前副领域与必要历史由 Rust 保存。现有直接领域读取已逐项分流：

| 当前消费者 | 源依据与职责 | 第四步处理 |
| --- | --- | --- |
| `player_abilities.rs:184 active_casting_realm_profiles` | 当前可学习/可施放书、能力覆盖、熟练度资格；`cmd5.c` | 改读当前副领域；第三领域只在合法改换学习入口可选，不能提前成为可施放领域 |
| `mogaminator.rs:926,933,1082,1086` | `autopick.c` First/SecondRealm 变量、领域谓词和 unreadable 分类 | 当前副领域替代出生第二领域；确认改换后即时刷新，旧书不再被视为当前书 |
| `loot/allocation.rs` 经 active profiles 的 needs-book / compatible book | `object2.c:3478` 主副高阶书 found 阈值分别参与需求 | 改换后按新副领域与已有发现统计判断；不要重置全局发现次数或只检查主领域 |
| `town.rs:1248` 领域 owner/member | `bldg.c` 主副任一匹配即具有领域资格 | 改换立即反映在 Life/Sorcery 公会价格/服务资格，不能只刷新名称 |
| `virtues.rs:137` | `virtue.c:441` 出生领域美德 | 保持出生读取；不在改换后重播出生美德 |
| `player_stats.rs:1967` | Priest 善恶领域下的刃器限制 | 当前只服务 Priest；Mage 不添加牧师惩罚。公共查询调整时保留其身份条件 |
| `inventory.rs:1304` | Paladin 高阶异阵营书销毁 | 主领域且限定 Paladin；不改为任意副领域，也不给 Mage 销毁奖励 |
| `loot.rs:426` guardian firstRealmBook | `quest.c:1333` Life 占位高阶奖励替换成主领域 | 继续读不可变主领域；副领域改变不影响奖励领域 |
| `town.rs:initial_town_and_shop_states` | `shop.c:2581,2601` 源初始店存为双方各补第二本书 | 当前 shop 按正式配置生成，无双方自动补书。第五步核实八领域书籍获取与初始配置；不因改换重置已生成店存或凭空补发书 |
| 保存、投影和角色信息 | 源 `py_info.c:132`、`files.c` 当前双方展示 | 当前 UI/DTO 无可变第二领域流程；第六步显示出生组合与当前副领域的实际意义，前端不推导规则 |

保存涉及 [Game 字段](../crates/rfb-core/src/game/mod.rs)、[persistence.rs](../crates/rfb-core/src/game/persistence.rs)、[validation.rs](../crates/rfb-core/src/game/validation.rs)及 player save DTO。当前保存/恢复只保留 Build 身份、learned IDs、learning order、AbilityProgress 和额外学习容量；不够表达改换历史及重复研习支出。源 `save.c:565,587,598,1415` / `load.c` 保存当前双方、64 项 prof、old realm、各状态位、learned_spells 和 order。

新增状态先还原当前领域，再验证书/法术和动态主副上限。拒绝与主领域相同、不支持、未知的副领域、旧领域残留进度、无效学习顺序和越界预算；不能从出生 Build 覆盖有效改换。已花次数不应被强制等于现存法术数：重复研习和历史副领域已消耗机会都使两者不同。状态进入 `player_save_dto` 和 state hash；第三/四步按变化升级协议/保存/哈希并生成绑定、运行完整 active 契约。只维护新开发存档。

间接源消费者中，`corny.c` 的 Craft 上限、`shop.c` 的 Burglary 特权、`effects.c` 的 Hex/Bard、`polly.c` 的 Politician 改领域、Rogue 专用手套和缺失四领域均不在本轮；源偏好文件加载由现有 Mogaminator/界面设置承担，不新增 C 的 `.prf` 加载系统。

## 5. 吞噬魔法

`mage.c`：25 级 INT 职业能力，cost 1、base fail 90；`spells_c.c:1423` 的 `eat_magic_spell` 最低失败率为 11。现有 `demo.ability.high-mage-eat-magic` 的 activation minimumFailurePercent 为 0，不能作为 Mage 正确配置的证据；复用 `rfb.ability-program.mutation.eat-magic` 和 [items.rs:831](../crates/rfb-core/src/game/abilities/items.rs)，新增 Mage 入口与最小失败率即可，不另造执行器。

`spells3.c:4358` 内部装置检查另算一次：power=`20+L×8/5`，difficulty=D，drain=`min(D,device_sp)`；odds=`(power−D/2)/5`（不满足条件时为 0）。`z-rand.h:74 one_in_` 对 odds≤0 返回真，因此该边界**必定失败**。当前执行器用 `odds>0 && rng==0`，错误地将 0 当作成功；第二步必须修复，并验证 mutation/High-Mage 真实调用者。

内部失败时固定神器只耗空；普通装置 90% 耗空、10% 摧毁一件；成功才恢复相应 MP。源 filter 只允许有 device SP 的 wand/rod/staff，位置是背包或脚下。当前 targeting 只检查 charges>0 和位置，应在真实内容边界核对装置类型/activation，不能让任何带 charges 的物品都可吞噬。当前 artifact 判定包括所有神器，源是 fixed artifact；若未来出现可吞噬随机神器装置，必须按真实表示区分，不能假设两者永远等同。

第二步证据须区分外层职业失败与内部装置失败，覆盖成功扣实际 SP、耗空、毁一件、固定神器保护、odds 0、无效/取消目标以及满 MP；RNG 与事件/保存中的装置 SP 一致。沿用本项目整型 charges 表示 device SP 的公共适配。

## 6. 生成、任务、公会与内容引用

| 源入口 | 当前消费者 / 缺口 | 第五步要求 |
| --- | --- | --- |
| `object2.c:_is_device_class` Mage 属装置偏好类 | [allocation.rs](../crates/rfb-core/src/game/loot/allocation.rs)有 High-Mage 专项，device kinds 55/65/66 和 tailored 额外 1/7 分支未含 Mage | 增加真实身份；先 1/10 needs-book 再 device 的顺序保持，不能泛化所有能用装置的职业 |
| `obj_kind.c:object_is_favorite`、`autopick.c:object_is_icky`、完成品 tailored rejection | Mage 默认 melee favorite，装备槽/手套负重影响完成品；公共路径已有 | 以 Mage caster 和实际种族装备槽验证；普通池不套 tailored 筛选，不能照搬 Duelist 盾牌/护甲限制 |
| `artifact.c:2230` 无外部 theme 的创造神器卷轴，在 1/4 职业 bias 路径 Mage 使用 Mage bias，Warrior 机会参数 20 | [random_artifact.rs](../crates/rfb-core/src/game/random_artifact.rs)仅 High-Mage 命中此组，Mage 会落入默认 Warrior | 补 Mage；自然/显式主题随机神器不全局套职业 bias，保持源 RNG 顺序和 virtue 路径 |
| `q_old_castle.txt:343–349` `$RANDOM27 MOD 10` 为 0→Gandalf、1→Saruman、其余→Indra | [middle-earth.json](../packs/rfb-demo-original/worlds/middle-earth.json)无 Mage/High-Mage 此奖励；正式 items 未导入三件。`tasks.rs` 当前预选与重复固定神器替代专门限 Duelist | 新增本批可达三件源神器、Mage 1:1:8 奖励、领取前后确定性、生成唯一性及重复替代；复用现有奖励/神器流程，不复制另一套 |
| `a_info` N:120/249/33；`artifact_name_zh.inc` 对应索引 | 权威中文后缀逐字为“甘道夫的”“萨鲁曼的”“因陀罗的”；前两件 wizardstaff，后一件 hard leather cap（TV_HELM/sval 2） | 组合显示名沿现有来源命名规则；导入属性、额外能力、甘道夫 INVULNERABILITY 90/777、萨鲁曼 RESISTANCE 25/111 及使用/保存，不只放名称和奖励权重 |
| `q_thieves.txt` 普通 Mage 默认 long sword；仅 SPEED=2 覆盖成 WAND_BOLT_ELEC | 当前默认 broad-sword，只有 Duelist 有职业覆盖 | 普通 Mage 补源 long sword；当前没有对应快速模式，不无条件送魔杖 |
| `t_angwil.txt:130`、`t_thalos.txt:199` Mage 塔 Owner | [angwil-mage-tower](../packs/rfb-demo-original/townFacilities/angwil-mage-tower.json)仅 High-Mage；Thalos 源 B:8 巫术之塔尚未映射，现有 royal-academy 是独立 quest-giver | 加 Angwil 资格并映射 Thalos 的真实服务/价格，不能把 royal-academy 改名充数 |
| `t_morivant/t_telmora/t_thalos` Life/Sorcery 领域 Owner | 现有 realm owner/member 数据及 `town.rs` 可复用，当前仍读出生双方 | 第四/五步用新副领域核对获得/失去服务；无源 Mage 资格的其他公会不自动赋予会员 |

三件神器已有权威中文名，不属于 unresolved；如果第二步后 `master` 变化，实施时重新记录实际来源提交。新增内容、引用和消费者由本批完成；公共装置、固定神器、任务或地点文件只有发生实际并发写入才协调，不作为预先审批点。

### 生成审计条件的可用性变化

已检查 [generation-build-applicability.json](../design/generation-build-applicability.json)全部 36 个 conditionScopes。当前 **没有** `unavailableClassIds` 明列 Mage；这不代表无需更新：`tailored-device-class` 等文字与实现仅覆盖 High-Mage，单纯通过 unavailable 检查会漏掉 Mage 的真实缺口。

| 条件组 | Mage 开放时的处理 |
| --- | --- |
| `tailored-device-class`、`tailored-class-hooks`、`artifact-scroll-class-bias` | 有新增职业分支，必须增加实现与行为证据 |
| `needs-book`、`tailored-compatible-kinds`、`great-book-count`、`good-book-count` | 双领域和改换改变输入；区分 Good/Great 数量限制与 Tailored 发现阈值，覆盖旧副领域退出、新副领域加入 |
| `equipment-category-weight`、`tailored-favorite` | 新 Class、双书和实际身体槽输入；记录 Mage 默认 favorite 与 caster 手套完成品筛选，无额外 Duelist 限制 |
| `artifact-scroll-virtues`、`artifact-theme-bias`、`theme-reset`、`base-theme` | 复用共同生成规则，以 Mage 身份验证正确入口；不把卷轴分支推广到自然/外部主题 |
| `fixed-artifact-identity` | 重核三件新增奖励神器及公共唯一性/随机替代证据；已有 unavailable 物品清单不能自动代表此次三件 |
| `ego-theme`、`bad-luck-randart`、`bad-luck-tomte-hat`、`bad-luck-fixed-special`、`bad-luck-fixed-normal`、`luck-quality-virtue`、`theme-jewelry-power`、`good-luck-fixed-retry`、`ring-allocation-weight`、`gold-virtue-personality`、`bikini-personality` | 无 Mage 专用新增源分支；继续按实际种族/性格/主题适用，复用共同证据，不删除跨种族条件 |
| `monster-ring-activation`、`mauler-weight`、`bard-slot-value`、`berserker-warning-no-tele`、`fixed-harp-bard`、`politician-gold`、`fixed-harp-pval`、`base-harp-bard`、`karrot-replacement`、`book-awareness`、`inspired-smithing` | 不因 Mage 开放改变其独立职业/种族/入口资格。尤其 book-awareness 的 Sorcerer/Red-Mage 和 inspired-smithing 缺失入口继续保留，不删除 unavailable 来过检查 |

第六步每个实际开放 Build 登记五个范围：基础/Tailored/Acquirement、Ego/负向生成、随机神器、固定神器/奖励、使用/保存。56 个记录引用共享来源和必要差异，不能复制 56 套同样测试。第五步结论先留本审计；正式 records 与第六步 `CREATION_BUILDS` 原子进入可用集合，届时更新 sharedReviews、conditionScopes、gap 双向关联并运行生成报告与 applicability 检查。本步不提前写 56 个“已可用”记录，也不生成报告。

## 7. 明确排除和公共适配

`monspell.c` 的 Mage 治疗倍率来自怪物 body class；`monster2.c` 的 SUMMON_MAGE 是怪物分类；眼魔、巫妖、戒指与附身中的 pseudo_class/Eat Magic 是独立怪物职业入口。普通 Mage 开放不自动开放这些玩家身份。其他职业专修领域、Law/Craft/Trump/Chaos、Coffee 模式与缺失 realm stone 仍属于各自范围。

保留的公共适配为：现有出生属性与 HP progression、整型装置 SP、先校验目标的原子施法命令、既有资源最大值刷新约定及源 `.prf` 的本项目替代。初始商店通过正式配置保障书籍获取，不在改换时重建店存。已经发现的 **Mage 法术参数、主副资格、减耗舍入、Death 反噬、练习、吞噬 odds=0、生成偏好和可达奖励**属于实施缺口，不能再用“公共适配”直接豁免。

## 8. 后续步骤退出证据

1. 第二步：全 56 出生引用、双方书与源职业参数；1/25/50 级及相关阈值；龙人/种族出生交叉、周期感知、真实 MP/负重、吞噬装置的外层/内部失败。参数类型变化才生成 Schema，内容变化更新包和 lock。
2. 第三步：主副学习/施放、重复研习付费、100 总预算边界、遗忘/恢复、1600/1400 上限、难度/深度练习与无效攻击、首用 XP、美德、失败反噬及减耗舍入。修公共代码时复用 High-Mage/Paladin/mutation 既有覆盖。
3. 第四步：确认拒绝、确认后取消法术、换回旧领域、不足预算、地面/背包书、当前书掉落与自动拾取/公会消费，以及保存后相同后续动作的状态和 RNG。保存校验不能把支出误等同已知数。
4. 第五步：实际 Mage 装置/Tailored/卷轴路径，三件固定神器奖励与重复替代、普通盗贼奖励、两座源 Mage 塔和领域公会、八领域书籍可获得性。证据应触发行为而非只断言数据存在。
5. 第六/七步：两层领域选择与 Rust 学习/改换投影，完整 56 Build 审计，再按计划做新存档自然流程和显式高等级准备、保存继续与 Tauri standalone。不得以内容数量或构建通过宣称自然升到 50 级/通关。

## 当前进度与验证

前四步已完成，第四步基线为 `bd2834bff`。正式定义见 [Mage Class](../packs/rfb-demo-original/classes/mage.json)、[技能](../packs/rfb-demo-original/skillSets/mage.json)、[玩家 actor](../packs/rfb-demo-original/actors/mage-player.json)、[职业能力](../packs/rfb-demo-original/abilities/mage-eat-magic.json)及 [Build 目录](../packs/rfb-demo-original/builds/)。56 个有序组合各携带双方第一本书，共用职业定义；未添加创角目录项。

- 复用 importer 的 `parse_m_info`，按 book rank/书内次序导入 256 项 Mage 等级、基础费用、失败率和首用经验；逐项对照解析结果通过。`firstSuccessExperience` 已存 `sexp × minimumLevel` 的最终值，运行时不再乘等级；死亡 Wraithform 的 250×47 和 Nature's Wrath 的 150×40 有实际首用奖励覆盖。
- 源属性/技能、武器熟练度（含双节棍 0/0）、400/100/20 攻击参数、MP/再生/负重、周期感知、美德与龙人变形等级已接入。保持公共出生属性和 HP progression 适配；没有复刻源点购或 HP 掷点曲线。
- `life-warding-true` 在主 Life 的等级为 46，副 Life 调整为 99；`death-vampirism-true` 的附加费用在公共有效参数路径处理，Mage 为 85、High-Mage 为 80，再进入减耗。死亡熵球与恶魔 `hellish-flame` 复用已有等级缩放。公共减耗现按源合并整数除法；非主 Life 的 warding glyph 限额为 11，主 Life 豁免，保留既有爆炸符文限额。
- 吞噬魔法复用现有装置执行器，目标要求真实装置、activation、有 SP 且在背包/脚下。odds=0 已按源必败，不掷内部成功骰；仍有外层职业失败骰及失败后的毁坏判定。Mage 和 High-Mage 的最低失败率为 11。既有 High-Mage/突变成功测试原先使用必败难度，现改为合法成功难度并明确选择成功 RNG；没有放宽断言。
- 共享学习预算使用独立 `spentSpellLearning`，首次学习与重复研习各付一次；首次顺序不重复，主领域上限 1600、副领域 1400。等级/INT 下降时从当前容量及已花预算重算遗忘，恢复时保留原顺序、进度和计数；Mage 不提供手动遗忘退款。研习按职业 `spell_book` 增 Knowledge，成功施法美德按实际领域；副领域 +5、阵营、美德、stun 和熟练度修正在源顺序内计算失败率。
- [book_magic.rs](../crates/rfb-core/src/game/abilities/book_magic.rs)在实际效果后判断练习：非攻击按三倍插值，攻击要求有效影响可移动敌方；城镇、空打、友方、不移动目标和免疫不刷伤害练习。地牢读实际深度，荒野读当前危险等级；环境上限与主副硬上限分别生效。成功但无用的攻击仍登记 worked/首用奖励，失败不成长。Death 反噬使用书的正式 rank 与书内 index，不能依赖被加载器排序过的书 ID 列表；复用精神冲击、直接失血及经验损失路径并保留 RNG 顺序。
- 学习支出为必需保存字段，非 Mage 必须为 0；恢复时校验支出、首次顺序、遗忘结果、全部进度和动态上限。Nature's Wrath 待方向状态保持未支付/未 worked，取消不成长；保存后确认方向与原存档继续完全一致，拒绝伪造待定进度和付款。保持上文公共目标校验、HP/MP 成长等适配，没有新增旧开发存档兼容。

核心证据为 [mage.rs](../crates/rfb-core/src/game/tests/mage.rs)、[learning.rs](../crates/rfb-core/src/game/tests/mage/learning.rs)及 [realm_change.rs](../crates/rfb-core/src/game/tests/mage/realm_change.rs)的 37 项行为测试：前三步出生/成长/学习覆盖，以及背包和脚下书本请求、取消确认、确认后停在选法术前、换回旧领域、预算耗尽、存档损坏拒绝和保存后的施法/物品生成继续。种族覆盖人类、托姆特、冬贝利、幽灵和红色龙人；不表示所有种族组合的桌面验收。领域资格边界继续复用 [内容校验测试](../crates/rfb-content/src/tests/validation.rs)。

- [spell_realms.rs](../crates/rfb-core/src/game/spell_realms.rs)保存当前副领域、旧领域集合和待确认书本 ID；出生 Build 与主领域不改。请求必须有可用预算及真实可读书本，确认期间仅接受确认/取消；确认清理旧副领域的首次顺序、已知/遗忘和全部进度，再初始化新领域。旧支出保留；清除遗忘项可能使预算归零，随后选法术失败也不回滚改换。旧领域集合用于历史展示，不恢复进度或禁止换回。
- 第 4 节列出的直接消费者已复核：active realm profiles、Mogaminator 第二领域变量/谓词及 unreadable、书本需求与实际分配、公会资格/价格改读当前副领域。Mogaminator 八领域中英文名采用源表。出生美德、静态 Build 校验、主领域奖励、Paladin 销书、主 Life 资格及 Priest 刃器分支保留各自原职责；不因改换重置商店或发现统计。
- 恢复先验证并还原当前领域，再验证全部法术引用、进度、学习顺序和预算。已换领域的累计支出不再受已删除进度的阶位总量约束，仍受历史最高容量与最多 64 个遗忘槽的上界及当前已学数量的下界约束。拒绝未知/不支持/主副相同的领域、非法历史、旧领域残留进度、越界支出和无效/冲突的待确认状态。前端只增加当前领域投影及待确认命令锁，选择界面留在第六步。

第四步实际检查：37 项 Mage、既有 High-Mage/Paladin/随机祈祷、Mogaminator、书本分配、城镇、保存、发现统计、神器身份和 Mindcrafter 相关回归通过；协议 7、保存容器 2 项通过。`rfb-core` / `rfb-protocol` / `rfb-save` all-targets Clippy、生成绑定检查、前端 typecheck 和 41 项状态/会话/背包/创角消费者测试通过。新增持久状态进入哈希，初始保存回环先 observe，再刷新 26 条 active 契约；diff 只有 stateHash/saveRoundTripStateHash，全部 verify 通过。内容未变化，不重编内容 Schema 或升级包/lock。

第五至七步仍待完成：完整职业生成/任务/公会关联、正式界面与桌面交付。本批没有 Tauri 或 Android 验收，也没有发布新的可玩程序；56 个 Mage 仍未进入普通创角或可用生成审计集合。
