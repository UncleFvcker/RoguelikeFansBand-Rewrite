# Warrens 怪物机制实现清单

状态：由 contract-v171 的 Warrens 生态对照建立；contract-v173 已完成 W1-W6 与运行时自然补怪，contract-v174-v176 已完成 W7-W9，contract-v177-v180 已完成 W10-W13，contract-v182 已完成 W14 Pest Control 任务生态；contract-v183 开始分批接入正式浅层内容，contract-v184 完成 `NEVER_MOVE` 与怪物 `BLINK` 绑定，contract-v185 接入第二批 14 只 2–3 级怪物，contract-v186 接入第三批 13 只 4–5 级怪物，contract-v187 接入首批正式施法怪物与简单 Unique，contract-v188 接入 10 只机制完备的 6–7 级怪物，contract-v189 接入 12 只 8–9 级怪物并完成浅层普查收口，contract-v190 完成出生宽限与五类职业掉落并再接入 13 只怪物，contract-v191 完成六类非伤害近战并再接入 10 只怪物，contract-v192 完成无近战怪物与 `SHRIEK`，contract-v193 完成地面拾物与四类近战偷窃/消耗事务，contract-v194 完成穿墙、水生、隐形与 Outpost 地表 habitat 分配，contract-v195 完成强者破体、两格近战、骑乘闭环与银质事实记录，contract-v196 完成友善娜美、怪物陷阱、Shadower 外观覆盖与废弃索引绑定，contract-v197 以零骰 `HURT` 和 `S_LOUSE` 收割 7 条浅层记录，contract-v198 以窄 `DISENCHANT` 接入解除附魔之眼，contract-v199 以持久房间 `glow`、黑暗源和 `darken-room` 接入银色果冻与黑暗精灵并完成浅层普查，contract-v200 复用现有机制直接接入 20 只十级非施法怪物，contract-v201 继续接入 7 只十级施法怪物并补齐通用召唤类别标签，contract-v202 以 `LIGHT → LITE` 源别名接入伪龙和明暗吐息资源，contract-v203 直接接入 6 只低风险十一级怪物，contract-v205 收口五类十一至十二级特殊机制，contract-v206 直接接入 20 只十二级非施法怪物，contract-v207 接入 7 只十二级施法怪物，contract-v208 接入全部 10 只十三级 P23 怪物，contract-v209 完成公共再生与 `MOVE_BODY`，contract-v210–v212 接入十四至十五级直接收割和参数化施法批次，contract-v213 以分类蚂蚁召唤和十格目标闪现接入 P28/P29，contract-v214 以窄单体 bolt 反射接入铁甲虫，contract-v216 直接接入 12 只十六级怪物，contract-v217 以蜘蛛分类召唤接入南蛮大王木鹿大王，contract-v218 完成剩余四只十六级阻塞怪物及其窄机制，contract-v219 直接接入 8 只十七级怪物，contract-v220 以参数化能力和 Dwarf 掉落表接入 5 只十七级怪物。

当前权威原版来源为 `master` Git ref 的 commit `efd63661302866038f58d8cd2553b23e6af3bf9d`。Warrens 在 `d_info.txt` 中为深度 1–9，主字形集合为 `kKyYrRfFcCbB`，并带有 `MONSTER_DIV_16`。本清单只记录该来源明确要求、而当前重写版还不能完整表达的机制，不把标签或近似行为标成已完成规则。

## 当前可玩边界

- Warrens 不再维护静态怪物白名单，而是从全局 actor 分配池过滤候选。主字形 `kKyYrRfFcCbB` 保留完整权重，其他合格怪物按原版 `MONSTER_DIV_16` 的 `16/64` 权重参与；`WILD_ONLY` 不进入普通地牢。
- 每次分配先执行两次独立 `1/40` 越级判定，每次按判定时等级增加 `min(5, level / 10) + 2`。因此 Warrens 9 层可沿 `9 -> 11 -> 14` 极低概率抽到 Warg；Warg 仍主要属于 Pest Control，而不是第 4 层起常驻怪物。
- leader 成功落位后才展开 `FRIENDS`/`ESCORT`。`FRIENDS(XdY)` 使用真实骰值且总数包含 leader，空间不足只缩减同伴；Mughash 的 escort 从同字形、低等级、非同种且阵营兼容的候选中选择。
- Unique 具有跨当前层、离层仓库、普通分配、分类/固定召唤共享的权威可用性；普通非 guardian Unique 死亡进入存档和 state hash。Mughash 仍由 guardian 生命周期管理，避免重复登记。
- Giant White Mouse 已进入全局分配并按 `MULTIPLY + RAND_50` 行动。繁殖受整层最多 100 个繁殖者约束；Warg 按 `RAND_25` 随机移动。随机移动发生在施法回退后、普通追踪前。
- Warrens 已按原版基础 `1/160` 接入运行时自然补怪，并应用深度修正；生成位置必须距玩家超过 25 格，自然补怪同样可以展开群体。
- 普通怪物出生时已按原版 HP 骰逐骰生成实例生命；`FORCE_MAXHP` 的 Mughash 固定使用满值 150。多段普通伤害、毒抗和 Rat-thing 的 `1_IN_9 + SCARE` 已接入现有能力管线。

## 实现队列

| 顺序 | 机制纵切 | 原版要求 | 完成边界 |
| --- | --- | --- | --- |
| W1 | 全局怪物分配池与地牢偏好 | 主字形怪物使用完整 `100 / rarity` 权重；其他合格怪物经 `MONSTER_DIV_16` 降为基础权重的 `16/64 = 1/4`，而非禁用；`WILD_ONLY` 不进入普通地牢 | **contract-v173 完成**：全局 actor 池过滤、原版 rarity 权重、字形降权和深度资格已有独立测试 |
| W2 | 越级分配 | 普通分配每阶段约 `1/40` 提升选择等级，最多连续两次 | **contract-v173 完成**：两次独立判定与当前等级 bonus 已锁定，9 层双越级可达 14 级 |
| W3 | 抽中领袖后展开群体 | 原版先抽到单个 leader，再按 `FRIENDS`/`ESCORT` 生成同伴；不能为保证整组落位而改变领袖被抽中的概率 | **contract-v173 完成**：leader-first，空间不足只缩减同伴，不撤销 leader |
| W4 | 群体骰与上限 | `FRIENDS(XdY)` 使用骰值而非均匀 min/max；Warg 的 `3d3` 总群体为 3–9，通用展开最多 32 只 | **contract-v173 完成**：dice、总数包含 leader、32 只上限和稳定逐只落位已实现 |
| W5 | Unique 与 Mughash escort | `UNIQUE` 在全局生命周期内只存活/生成一次；Mughash 的 `ESCORT` 从同字形、合格低等级怪物中选择 | **contract-v173 完成**：Unique 权威状态、跨层可用性、共享召唤过滤和原版 escort 候选均已实现 |
| W6 | 繁殖与随机移动 | Giant White Mouse 具有 `MULTIPLY`、`RAND_50`；Warg 具有 `RAND_25` | **contract-v173 完成**：全层繁殖者上限 100，睡眠检查后尝试繁殖；RAND 移动在施法回退后、追踪前执行 |
| W7 | 怪物门交互 | Kobold 系列与 Mughash 可 `OPEN_DOOR/BASH_DOOR`，Wild Cat/Warg 可破门 | **contract-v174 完成**：原版 HP/power 判定、开门/解锁分步行动、同回合撞门和 50% 破损已进入统一移动事务 |
| W8 | 移动域与地形关系 | Newt/蜥蜴可游泳，Fruit Bat/猎鹰可飞；飞行、游泳应影响可通行地形、危险地形和陷阱 | **contract-v175 完成**：强类型 movement profile 已统一用于路径、落位、召唤、位移、读档和显式 trap avoidance |
| W9 | HP 骰与强制满 HP | 普通怪物从 HP 骰生成个体生命，`FORCE_MAXHP` 固定取骰面满值 | **contract-v176 完成**：所有出生入口共用逐骰/强制满值 helper，实例上限保存后不重掷 |
| W10 | 特殊近战效果 | 完整生态需要毒、疾病、属性损伤、流血等 blow effect，以及爆炸攻击和攻击者自毁 | **contract-v177 完成**：有序 effect、独立概率、抗性/护甲、致死中断，以及任意死亡触发首个 `EXPLODE` blow 的半径 3 投射 |
| W11 | 怪物改变地图与物品 | `KILL_WALL/KILL_ITEM` 等旗标会改变追踪路径与地图/地面物品状态 | **contract-v178 完成**：怪物专属破墙寻路/变换、地面物与金币销毁，以及 artifact、匹配 slay/brand、Endurance 弹药保护 |
| W12 | 怪物光源 | `HAS_LITE/SELF_LITE` 影响玩家可见区和怪物自身感知 | **contract-v179 完成**：typed actor light 进入权威光照；睡眠抑制 `HAS_LITE`，不抑制 intrinsic `SELF_LITE` |
| W13 | 完整掉落旗标与主题 | `DROP_60/90`、`DROP_1D2`、`ONLY_ITEM`、`DROP_GOOD`、职业主题和尸体/骨骸需要统一组合 | **contract-v180 完成**：次数概率/骰、压缩、20% 金币、50% 职业主题、only kind、质量下限与 remains 顺序可组合 |
| W14 | Pest Control 专属 Warg 生态 | Warg 主要属于城镇 Pest Control 任务，而不是 Warrens 4–9 层常驻怪物 | **contract-v182 完成**：接取后在 Warrens 5 创建剩余目标 Warg；任务目标严格为 8 只，`FRIENDS(3d3)` 仅是 Warg 的普通生态群体规则，不是任务数量；Warg 沿用 `RAND_25`，完成后生成魔法楼梯，未完成离层失败并丢弃任务层，回城领取毛皮披风 |

## 推进原则

- 每个 W 项单独做最小 fixture 或纯单元测试；非移动机制不通过移动命令搭建前置条件。
- 通常不为一次生态扩充刷新全部 contract fixtures，只刷新实际受输出变化影响的 `dungeon`、`campaign` 或 `monsters` 分类。contract-v173 因 state-hash 输入新增 Unique 权威状态而按已批准规则一次性刷新全部分类。
- 新 actor 可以先以已支持的攻击、抗性和施法进入内容包，但所有被省略的原版旗标必须留在本清单，不能靠标签假装规则已完成。
- W1–W14 与自然补怪已经完成。W14 的内容和运行时边界见 [Contract v182](contract-v182-pest-control.md)。

## contract-v183 正式内容批次的保留旗标

contract-v183 通过当前 `master` Git 对象接入 12 只浅层怪物。W1-W13 所
覆盖的分配、群体、繁殖、随机移动、门、移动域、HP 骰、近战、爆炸、
光照、掉落与尸骨均已正式表达。以下不属于 W1-W13 的原版旗标继续保留
为后续缺口，不改变当前 Warrens 战斗和分配边界：

- `WEIRD_MIND`：Giant White Centipede、Green Worm Mass、Grid Bug、Soldier
  Ant、Insect Swarm、Bomb Mosquito；
- `EMPTY_MIND`：White Icky Thing；
- `STUPID`：Green Worm Mass、Grid Bug；
- `POS_GAIN_AC`：Giant White Centipede、Green Worm Mass、Grid Bug、Insect
  Swarm、Bomb Mosquito；
- `WILD_*` habitat：等待正式荒野分配系统，不进入普通地牢资格判定。

contract-v184 已将 Grey Mold 的 `NEVER_MOVE` 建模为禁止自主物理移动但保留
相邻近战、施法和位移，并将 Blinking Dot 的 `BLINK` 绑定为半径 10 的正式
怪物“闪现”能力；两只怪物均已进入全局分配。它们仍保留以下尚未实现的
非行动旗标：Grey Mold 的 `STUPID`、`EMPTY_MIND`、`POS_GAIN_AC`，以及
Blinking Dot 的 `STUPID`、`EMPTY_MIND`、`NASTY_GLYPH`、`POS_GAIN_AC`。

contract-v185 通过严格选择清单继续接入 14 只 2–3 级怪物；所有 W1-W13
行为已经正式表达，剩余非行动旗标按原版记录如下：

- `WEIRD_MIND`：Metallic Green Centipede、Giant Black Ant、Slimy Worm Mass、
  Cave Spider、Metallic Blue Centipede、Giant White Louse、Giant White Ant、
  Metallic Red Centipede、Yellow Worm Mass；
- `STUPID`：Slimy Worm Mass、Slimy Ooze、Spotted Mushroom Patch、Yellow Mold、
  Yellow Worm Mass；
- `EMPTY_MIND`：Slimy Ooze、Spotted Mushroom Patch、Yellow Mold；
- `POS_GAIN_AC`：Metallic Green Centipede、Slimy Worm Mass、Cave Spider、Slimy
  Ooze、Metallic Blue Centipede、Giant White Louse、Yellow Mold、Metallic Red
  Centipede、Yellow Worm Mass；
- `WILD_ALL/WILD_GRASS/WILD_WOOD/WILD_VOLCANO/WILD_SWAMP`：等待正式荒野
  habitat 系统，不改变普通地牢分配资格。

contract-v186 继续接入 13 只 4–5 级怪物。其分配、繁殖、随机移动、群体
概率、门、移动域、光源、爆炸、近战、抗性、掉落与尸骨均已正式表达；
保留项只包括 `MALE`、`COLD_BLOOD`、`STUPID`、`EMPTY_MIND`、`WEIRD_MIND`、
`POS_GAIN_AC/POS_HOLD_LIFE` 与 `WILD_*` 元数据。需要主动行为的同级怪物
继续后置，不用 omittedFlags 隐去 aquatic-only、穿墙、拾物、偷窃、骑乘、
主动施法或 Unique 特例。

contract-v187 允许正式怪物选择器复用已经实现的 `DRAIN_MANA`、`SHOOT`、
`CAUSE_1`、`S_UNDEAD`、`BR_SOUND`、`BLIND/SLOW/CONFUSE/SCARE` 映射；频率、
参数和声明顺序直接来自原版 `S:` 行，未映射 token 仍会使同步失败。首批
简单 Unique 只使用现有一次性生成/击杀状态和完整可表达的近战、门、光照、
掉落与尸骨行为。保留项为性别、可说话、特殊心智/附身提示和 `WILD_*`
habitat 元数据；带偷窃、荒野专属、专属物品、未支持特殊近战或 `S_LOUSE`
等未映射召唤的 Unique 不在本批选择中。

contract-v188 覆盖原版全部 29 条 6–7 级记录并接入其中 10 条新增的机制完备
记录；连同此前已有的 8 条，同级已有 18 条正式内容。紫蘑菇丛的三段孢子
体质吸取、掐死人的断手、巨型褐蝠、响尾蛇、两类复生尸体、木蜘蛛、原魔、
粉红果冻和腐蚀恶心物均保留完整近战、移动、群体、光源、抗性与分配规则。
其余 11 条因睡眠 AI、骰值为空的状态近战、银质交互、`KILL_BODY`、
`WILD_ONLY`、穿墙、拾物或职业掉落主题而继续后置。

contract-v189 覆盖原版全部 45 条 8–9 级记录并接入其中 12 条新增的机制完备
记录；连同此前已有的 7 条，同级已有 19 条正式内容。新增记录保留群体、
繁殖、随机移动、门、飞行、破墙/毁物、光源、抗性、掉落、尸骨和有序近战。
其余 26 条因偷窃/拾物、睡眠 AI、穿墙、骑乘/水生或荒野限定、骰值为空的
状态近战、职业掉落、纯远程 `NEVER_BLOW` 或 `S_LOUSE` 而继续后置。

P1–P5 至此已枚举原版全部 173 条 1–9 级记录。正式包中 95 个 actor 带有
对应浅层原版索引，其中 63 个由严格选择/同步路径维护；其余 78 条保留在
批次文档和本清单的明确机制边界中。浅层内容普查里程碑已经收口，机制缺口
本身仍按共享运行时能力逐项推进。

contract-v190 实现 `FORCE_SLEEP → MFLAG_NICE` 的一次玩家行动出生宽限，并
建立 Mage、Archer、Priest、Evil Priest、Paladin 五张浅层职业掉落表。新手
巫师、新手牧师、新手弓箭手、新手游侠、巨型火蜥蜴、新手圣武士、兽人萨满、
五种幼龙和斯卡文萨满共 13 条进入严格同步。正式浅层 actor 增至 108 条，
严格同步增至 76 条，剩余 65 条继续等待各自真实机制。

contract-v191 将 `BLIND`、`CONFUSE`、`PARALYZE`、`SLOW`、`STUN`、
`TERRIFY` 建模为有序近战 effect，保留逐 effect 独立概率，并统一应用到玩家
和 actor 目标。漂浮眼、黄蘑菇丛、褐霉菌、充血的眼睛、史纳加队长拉格杜夫、
绿霉菌、眼镜王蛇、破碎死亡之剑、冷酷的巴尔克梅格和巨蛾共 10 条进入严格
同步；闪烁的圆点原有混乱 blow 同时修正为伤害与状态。正式浅层 actor 增至
118 条，严格同步增至 86 条，剩余 55 条继续等待各自真实机制。巨型蛞蝓、
空间怪物、僧帽水母和骚灵仍分别被 `KILL_BODY`、穿墙、水生限定或地面拾物
等独立能力阻塞，不以局部导入冒充完整接入。

contract-v192 将原版 `NEVER_BLOW` 表达为显式空近战 routine，并把 `SHRIEK`
映射到排除施法者的范围唤醒与视线内敌对怪物加速。尖叫蘑菇丛、白鹰身
女妖和射石兽进入严格同步；`DARKNESS` 与 `TRAPS` 继续分别等待房间暗化和
怪物陷阱生成基础设施。

contract-v193 将 `TAKE_ITEM` 接入移动后地面拾物，并把 `EAT_GOLD`、
`EAT_ITEM`、`EAT_FOOD`、`EAT_LITE` 接入有序近战。普通物品由怪物携带并在
死亡时沿统一掉落事务返还；神器、尸骨、雕像和会以 slay/brand 伤害怪物的
物品不会被拾取。偷金与偷物保留原版敏捷/等级保护、金币公式和偷窃后闪现，
食物与非神器光源燃料按单次命中扣除。小香雪兰、斯密戈、哥布林、绿娜迦、
粉红娜迦、小魔怪、吼牛者霍比特人和库塔熊进入严格同步，使正式浅层 actor
增至 129 条、严格同步增至 97 条，剩余 44 条继续等待各自真实机制。巧言、
罗宾汉和奈美仍被 `TRAPS` 阻塞，不能仅凭偷窃能力提前导入。

contract-v194 将 `PASS_WALL` 与 `AQUATIC` 建模为彼此独立的移动域：前者只
穿过明确可穿越的非永久墙体，后者只能落位水域，飞行水生怪保留飞越能力；
寻路、移动、召唤和离层落位共享同一判定。`INVISIBLE` 进入权威视野投影，
`see-invisible` 按原版搜索技能公式判定并保存看破状态。Outpost 通过草地、
城镇路径、疏林、岸边/沼泽浅水和深水承载 `WILD_*` habitat 与 aquatic 分配。
19 条记录进入严格同步，使正式浅层 actor 增至 148 条、严格同步增至 116 条，
剩余 25 条继续等待骑乘、特殊召唤、主动陷阱/暗化、银质交互和其他独立机制。

contract-v195 将 `KILL_BODY` 接入寻路与 actor 对 actor 近战，将
`RANGED_MELEE` 限定为两格直线/偏轴的干净线路，并建立 `RIDING` 的邻格命令、
坐骑移动/速度、楼层跟随、死亡清理和存档闭环。`SILVER` 只记录材质事实，
当前没有银脆弱角色，因而不增加无调用方的伤害钩子。哭闹的恶心物、新手
考古学家、爬行银币、巨型鼻涕虫、马、难以驯服的马和绵羊进入严格同步，
恰克波补上可骑乘事实。正式浅层 actor 增至 155 条、严格同步增至 123 条，
剩余 18 条继续等待特殊召唤、主动陷阱/暗化和其他独立机制。

contract-v196 将 `FRIENDLY` 接入现有阵营目标与清层判定，让航海士娜美
自主攻击敌对怪物并复用既有治疗、偷窃和拾物；`TRAPS` 直接复用半径 1 的
地形转换与兽穴陷阱。追踪者只作为 10 级以上非 Unique 普通分配怪物的
`1/333` 外观覆盖，真实种类不变并随存档持久化；板栗崽进入普通浅层分配。
正式浅层 actor 增至 158 条、严格同步增至 126 条。5 条 `DEPRECATED` 记录
永久绑定到活跃同名替代索引，剩余 10 条活跃浅层记录不在本批扩张范围。

contract-v197 只补两个窄缺口：无骰 `HURT` 作为受护甲减免的精确 `0d0`
进入既有 damage effect，`S_LOUSE` 作为既有 `summon-category` 的 `1d3+1`
映射，并以巨型白虱的唯一 `louse` 标签锁定候选。高阶地狱兽、黄色果冻、
佐格虫、巧言、罗宾汉、虱子王劳西和鸭子进入严格同步；正式浅层 actor
增至 165 条、严格同步增至 133 条。剩余活跃浅层记录仅为 Silver jelly、
Disenchanter eye 和 Dark elf，继续等待各自真实能力边界。

contract-v198 将无骰 `DISENCHANT` 映射为独立近战 effect，按原版 4:1
选择当前已建模正面时效或已装备武器/护甲/弹药，并复用 Disenchant 抗性、
状态移除、物品强化和神器抵抗。解除附魔之眼进入严格同步；正式浅层 actor
增至 166 条、严格同步增至 134 条。剩余活跃浅层记录仅为 Silver jelly 和
Dark elf。

contract-v199 为程序化房间逐格初始化持久 `glow`，并以目标格所在的八连通
发光区域作为 `DARKNESS` 的清除边界。银色果冻的半径 1 黑暗源只压制永久
房间光，玩家火把和怪物主动光继续照明；黑暗精灵复用既有施法选取与投射线
结算，但该能力不要求 line of effect。两只怪物进入严格同步；正式浅层 actor
增至 168 条、严格同步增至 136 条，剩余活跃浅层记录清零。

contract-v200 开始十级段直接收割：20 只非施法记录全部复用既有水生/飞行、
门、群体、繁殖、随机移动、Unique、爆炸、近战状态、掉落、尸骨、抗性和
habitat 语义，严格同步增至 156 条。猞猁保留 `WILD_ONLY`，不进入普通地牢；
其余 19 条进入现有全局分配池。特殊心智、附身提示和性别等非行动旗标继续
由严格清单的 `omittedFlags` 明示，不扩建无消费者的通用系统。

contract-v201 让 7 只十级施法怪物复用既有 bolt/ball、状态、治疗、位移、
召唤和怪物施法选择。demo 严格 actor 保留 `legacy-import` 标签，使
`S_MONSTER(1d1)` 可以从完整正式导入池选取，而不是生成无候选能力；该标签
与完整导入器原有语义一致。严格同步增至 163 条，未增加 effect 或协议类型。

contract-v202 仅在近战元素解析处接受原版使用的 `LIGHT` 拼写，并将其归一为
现有 `LITE` 对应的 `light` 伤害。伪龙复用既有飞行、破门、抗性、施法和
生命比例吐息 effect；明暗吐息各生成一个半径 2 的参数化资源。严格同步增至
164 条，未增加运行时伤害类型、协议字段或兼容层。

contract-v203 开始十一级内容：多彩龙幼龙、锋锐兔、马头鱼尾怪、僵尸兽人、
浅水洼和怪诞者卢格全部由既有字段完整承载。多彩龙幼龙复用五种已有吐息，
锋锐兔复用闪现，其余复用水生、骑乘、Unique、掉落、抗性和近战状态路径；
严格同步增至 170 条，没有新增 ability、effect、协议字段或兼容层。

contract-v205 以五条互不扩张的机制边界继续推进：`DRAIN_EXP` 只损失当前经验
并重算当前等级，历史经验/等级和属性点奖励不倒退；`A:POISON` 在成功接触命中
后复用毒抗、免疫和延迟中毒；`SHAPECHANGER` 只改变既有外观投影；
`DROP_WARRIOR_SHOOT` 复用弓手掉落表；`DUNGEON_31/35` 通过稳定原版地牢索引
过滤全局分配。混沌变形者、葡萄果冻、瘟疫鼠、骑士弓箭手、南蛮大王朵思
大王和小袋鼠进入严格同步，既有黏糊糊的软体补回 `1d1` 接触光环；严格同步
增至 176 条。瘟疫鼠 `COMPOST` 继续等待下水道任务消费者。

contract-v206 复用上述既有边界接入 20 只十二级非施法怪物；严格同步增至
196 条，ability 保持 116。雪人、灰熊与两类蠕虫团的 `S:BERSERK` /
`S:MULTIPLY` 是附身者专用提示，不进入怪物施法；蠕虫团真正的繁殖生态仍由
`F:MULTIPLY` 映射。没有新增 effect、协议字段、存档字段或兼容层。

contract-v207 接入 7 只十二级施法怪物；相同参数共享现有 ability，不同参数
生成 8 条稳定内容记录。严格同步增至 203 条、ability 增至 124；没有新增
effect、怪物参数覆盖、协议字段、存档字段或兼容层。

contract-v208 接入 10 只十三级怪物；严格同步增至 213 条，新增治疗、自我加速、
博尔多同类召唤和两种元素箭共 5 条稳定内容记录，ability 增至 129。全部复用
既有 effect、怪物字段和施法路径，协议、存档与 state hash 结构不变。

contract-v209 单独收口黏菌阻塞的两个真实机制。`MOVE_BODY` 只在来源经验值更高、
同阵营且双方均能穿越换位后的地形时交换位置，并唤醒被交换者；公共怪物再生
每 100 world ticks 执行一次，低 HP 保留单次二分 RNG，`REGENERATE` 翻倍并在
翻倍后封顶 400。黏菌进入严格同步，ability 增至 130；协议、存档与 state hash
结构不变。

contract-v210 直接收割 19 只无需新机制的十四级怪物，严格同步增至 233 条，
actor 增至 298，ability 保持 130。十四级剩余阻塞保持拆分：座狼需把 Pest
Control 手写 actor 与权威身份合并；瘟疫武僧、斯卡文刺客等待 `COMPOST` 的
真实下水道任务消费者；祝融夫人、火焰乌鸦等待火焰接触光环；维护者等待
`POLYMORPH` 与软件漏洞定点召唤。以上均不以重复 actor、通用触发框架或局部
近似规则代替。

contract-v211 直接收割 23 只无需新机制的十五级怪物，严格同步增至 256 条，
actor 增至 321，ability 保持 130。幻术师、光明/暗影猎犬、时间学徒和
鸭鸣鸭嘴兽复用已有能力；`DETECT_MONSTERS`、`BERSERK` 与 `MULTIPLY` 仍按
附身者提示处理，不生成怪物施法能力。

contract-v212 接入 8 只十五级参数化施法怪物，严格同步增至 264 条，actor
增至 329，ability 增至 141。11 条新 ability 只为既有 effect 固定权威参数，
寒冰箭、十五级单体召唤和 `heal-45` 由多个怪物共享；`BLESS` 与 `HEROISM`
仍按附身者提示处理。

contract-v213 接入纳垢携疫者和侏儒法师，严格同步增至 266 条，actor 增至
331，ability 增至 143。`S_ANT` 生成现有分类召唤参数；`BLINK_OTHER` 新增
怪物专用 `blink-target` 窄 effect，在目标当前位置半径 10 内按稳定候选顺序
抽取一个可通行空格，不复用远距放逐。

contract-v214 接入铁甲虫，严格同步增至 267 条，actor 增至 332，ability 保持
143。`REFLECTING` 记录为 `reflectsBolts`，只拦截单体 ability/device bolt；
75% 反射、十次方向重选、玩家/怪物命中与无奖励死亡复用现有 RNG、线路和伤害
事务，不建立通用投射重定向框架。

contract-v216 直接接入 12 只十六级怪物，严格同步增至 279 条，actor 增至 344，
ability 保持 143。粉红惧妖、兽人队长和巨型绿蜻蜓复用既有状态、物理投射物
和毒素吐息；其余复用酸性近战、地面物品破坏/拾取、繁殖、水生、群体、光源、
Unique、区域过滤和职业掉落。`DETECT_OBJECTS`、`MULTIPLY` 与 `BERSERK` 的
附身者提示不生成怪物能力。

contract-v217 接入南蛮大王木鹿大王，严格同步增至 280 条，actor 增至 345，
ability 增至 145。`S_ANT` 与新增的 `S_SPIDER → spider` 都复用现有
`summon-category`，只生成十六级上限和 `1d3+1` 数量参数，不增加新 effect。

contract-v218 接入恐爪怪、夸塞魔、老鼠王子尼祖基尔和布伦比野马，严格同步
增至 284 条，actor 增至 349，ability 增至 147。`HURT_ROCK` 复用解离易伤，
`CAN_CLIMB` 只扩展山地/冰川通行，`TELE_LEVEL` 复用现有楼层事务，`COMPOST`
通过 allocation `taskId` 限定真实下水道任务；完整任务内容没有混入本批次。

contract-v219 直接接入 8 只十七级怪物，严格同步增至 292 条，actor 增至 357，
ability 保持 147。斯芬克斯和2头海德拉复用恐惧/混乱施法，其余复用再生、
游泳/飞行、`MOVE_BODY`、近战恐惧/力量损伤、解离易伤、区域过滤和 Warrior
掉落。水元素精灵按权威记录保留随机移动、飞行和非生命体，不获得穿墙。

contract-v220 接入丘陵巨人、小恶魔、猫又、灰先知和矮人纳尔，严格同步增至
297 条，actor 增至 362，ability 增至 152。五条 ability 只承载现有伤害、
召唤和治疗 effect 的原版参数；`DROP_DWARF` 只增加一张引用现有物品的窄掉落
表。十七级剩余缺口仅为冰冻球、跳跃火球和球状闪电的元素接触光环。
