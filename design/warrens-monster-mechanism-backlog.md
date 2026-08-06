# Warrens 怪物机制实现清单

状态：由 contract-v171 的 Warrens 生态对照建立；contract-v173 已完成 W1-W6 与运行时自然补怪，contract-v174-v176 已完成 W7-W9，contract-v177-v180 已完成 W10-W13，contract-v182 已完成 W14 Pest Control 任务生态。

固定原版来源为 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`。Warrens 在 `d_info.txt` 中为深度 1–9，主字形集合为 `kKyYrRfFcCbB`，并带有 `MONSTER_DIV_16`。本清单只记录该来源明确要求、而当前重写版还不能完整表达的机制，不把标签或近似行为标成已完成规则。

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
