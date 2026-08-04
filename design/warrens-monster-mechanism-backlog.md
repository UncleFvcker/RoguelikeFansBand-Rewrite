# Warrens 怪物机制实现清单

状态：由 contract-v171 的 Warrens 生态对照建立，作为后续怪物机制纵切的实施队列。

固定原版来源为 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`。Warrens 在 `d_info.txt` 中为深度 1–9，主字形集合为 `kKyYrRfFcCbB`，并带有 `MONSTER_DIV_16`。本清单只记录该来源明确要求、而当前重写版还不能完整表达的机制，不把标签或近似行为标成已完成规则。

## 当前可玩边界

- 普通遭遇已接入 Newt、Small Kobold、Rock Lizard、Fruit Bat、Wild Cat、Kobold、Cave Lizard、Large Kobold、Rat-thing、Night Lizard、Hunting Hawk of Julian 和 Chiokovo。
- 静态表使用原版基础分配权重 `100 / rarity`，并从怪物原版等级对应的 Warrens 深度开始出现。
- 普通怪物 HP 暂取原版 HP 骰的确定性平均值；`FORCE_MAXHP` 的 Mughash 使用满值 150。多段普通伤害、毒抗和 Rat-thing 的 `1_IN_9 + SCARE` 已接入现有能力管线。
- Giant White Mouse 保留内容定义但暂不进入普通表，因为 `MULTIPLY + RAND_50` 是其生态身份；Warg 保留给 Pest Control，不再从第 4 层常驻生成。
- Mughash 仍作为最终层固定 guardian 存在，但当前还没有原版 escort 展开和跨来源唯一生命周期。

## 实现队列

| 顺序 | 机制纵切 | 原版要求 | 完成边界 |
| --- | --- | --- | --- |
| W1 | 全局怪物分配池与地牢偏好 | 主字形怪物使用完整 `100 / rarity` 权重；其他合格怪物经 `MONSTER_DIV_16` 降为基础权重的 `16/64 = 1/4`，而非禁用；`WILD_ONLY` 不进入普通地牢 | encounter selector 能从全局 actor 池按地牢过滤/降权，并以确定性测试固定候选与 RNG 次数 |
| W2 | 越级分配 | 普通分配每阶段约 `1/40` 提升选择等级，最多连续两次 | 把越级提升放在候选深度计算前；零候选回退、两阶段抽取顺序和上限进入最小单元测试 |
| W3 | 抽中领袖后展开群体 | 原版先抽到单个 leader，再按 `FRIENDS`/`ESCORT` 生成同伴；不能为保证整组落位而改变领袖被抽中的概率 | leader 选择与 companion 落位拆成两个事务；空间不足只缩减同伴，不撤销 leader |
| W4 | 群体骰与上限 | `FRIENDS(XdY)` 使用骰值而非均匀 min/max；Warg 的 `3d3` 总群体为 3–9，通用展开最多 32 只 | 内容 schema 表达 dice；稳定逐只落位；数量、空间缩减和 RNG 消耗各有独立测试 |
| W5 | Unique 与 Mughash escort | `UNIQUE` 在全局生命周期内只存活/生成一次；Mughash 的 `ESCORT` 从同字形、合格低等级怪物中选择 | 唯一状态进入权威世界状态和存档；guardian、召唤、普通分配共享过滤；Mughash 护卫不硬编码 actor ID |
| W6 | 繁殖与随机移动 | Giant White Mouse 具有 `MULTIPLY`、`RAND_50`；Warg 具有 `RAND_25` | 繁殖受相邻空间、种群上限和稳定调度约束；随机移动与追踪决策明确组合顺序；完成后再启用 Giant White Mouse |
| W7 | 怪物门交互 | Kobold 系列与 Mughash 可 `OPEN_DOOR/BASH_DOOR`，Wild Cat/Warg 可破门 | AI 将开门/撞门作为真实行动，复用权威 terrain 事务和能量成本，不绕过锁、阻挡与事件 |
| W8 | 移动域与地形关系 | Newt/蜥蜴可游泳，Fruit Bat/猎鹰可飞；飞行、游泳应影响可通行地形、危险地形和陷阱 | actor movement profile 统一用于路径、落位、追逐和陷阱触发；`flying`/`swimming` 标签不再只是分类信息 |
| W9 | HP 骰与强制满 HP | 普通怪物从 HP 骰生成个体生命，`FORCE_MAXHP` 固定取骰面满值 | 出生时只掷一次并保存实例 HP 上限；召唤、群体、guardian 与读档共用；新存档无需兼容旧开发存档 |
| W10 | 特殊近战效果 | 完整生态需要毒、疾病、属性损伤、流血等 blow effect，以及爆炸攻击和攻击者自毁 | `MeleeRoutine` blow 支持有序 effect 列表、抗性/豁免、致死中断和来源归属；爆炸不伪装成普通多段伤害 |
| W11 | 怪物改变地图与物品 | `KILL_WALL/KILL_ITEM` 等旗标会改变追踪路径与地图/地面物品状态 | 怪物行动复用地形变换和物品销毁事务，保护边界、连接和任务物品，事件可观察 |
| W12 | 怪物光源 | `HAS_LITE/SELF_LITE` 影响玩家可见区和怪物自身感知 | 动态 actor 光源进入权威光照/FOV 增量，不只用表现层发光 |
| W13 | 完整掉落旗标与主题 | `DROP_60/90`、`DROP_1D2`、`ONLY_ITEM`、`DROP_GOOD`、职业主题和尸体/骨骸需要统一组合 | 出生携带和死亡掉落共享一次内容驱动生成；次数骰、质量、仅物品、主题、金币替换和 unique 过滤均可组合 |
| W14 | Pest Control 专属 Warg 生态 | Warg 主要属于城镇 Pest Control 任务，而不是 Warrens 4–9 层常驻怪物 | 任务接受后创建专属目标种群，按 `FRIENDS(3d3)` 与 `RAND_25` 行动；完成、离场、重接和计数由任务状态机管理 |

## 推进原则

- 每个 W 项单独做最小 fixture 或纯单元测试；非移动机制不通过移动命令搭建前置条件。
- 不为一次生态扩充刷新全部 contract fixtures。只刷新实际受输出变化影响的 `dungeon`、`campaign` 或 `monsters` 分类。
- 新 actor 可以先以已支持的攻击、抗性和施法进入内容包，但所有被省略的原版旗标必须留在本清单，不能靠标签假装规则已完成。
- W1–W6 是 Warrens 生成真实性的优先路径；W7–W13 是可复用的怪物公共能力；W14 在 Outpost 任务服务线开始时实施。
