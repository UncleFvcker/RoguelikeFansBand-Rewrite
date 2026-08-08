# Warrens 怪物机制实现清单

状态：由 contract-v171 的 Warrens 生态对照建立；contract-v173 已完成 W1-W6 与运行时自然补怪，contract-v174-v176 已完成 W7-W9，contract-v177-v180 已完成 W10-W13，contract-v182 已完成 W14 Pest Control 任务生态；contract-v183 开始分批接入正式浅层内容，contract-v184 完成 `NEVER_MOVE` 与怪物 `BLINK` 绑定，contract-v185 接入第二批 14 只 2–3 级怪物，contract-v186 接入第三批 13 只 4–5 级怪物，contract-v187 接入首批正式施法怪物与简单 Unique，contract-v188 接入 10 只机制完备的 6–7 级怪物，contract-v189 接入 12 只 8–9 级怪物并完成浅层普查收口，contract-v190 完成出生宽限与五类职业掉落并再接入 13 只怪物，contract-v191 完成六类非伤害近战并再接入 10 只怪物，contract-v192 完成无近战怪物与 `SHRIEK`，contract-v193 完成地面拾物与四类近战偷窃/消耗事务。

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
