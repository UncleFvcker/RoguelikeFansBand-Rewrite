# rfb-core 测试缩减：第三步执行记录

日期：2026-09-09。基线为[第二步](core-test-reduction-stage2.md)完成后的 875 项。范围为世界生成、掉落、设施和任务测试。

## 实际结果

Core **875 → 871 项**。8 个旧测试合并为 4 个机制测试，另有 6 项按代表层范围改名；没有把改名算作删除，没有新增 ignored 测试或迁移到其他 crate。约 490 项预算没有达到；遵循用户“没有其他能删的就不删”的要求，不为数量继续移除专项保障。

| 文件 | 测试数 | 源码行数变化 |
|---|---:|---:|
| [crates/rfb-core/src/game/tests/tasks.rs](../crates/rfb-core/src/game/tests/tasks.rs) | 37 → 35 | +8 |
| [crates/rfb-core/src/game/tests/town.rs](../crates/rfb-core/src/game/tests/town.rs) | 40 → 39 | -20 |
| [crates/rfb-core/src/game/tests/world.rs](../crates/rfb-core/src/game/tests/world.rs) | 66 → 65 | -63 |

三个测试文件合计净减 75 行。主要收益是减少运行准备和抽样，并非入口数：

| 工作量 | 改前 | 改后 |
|---|---:|---:|
| 11 项布局测试中的地牢层生成 | 224 层 | 73 个代表层 |
| 9 项守卫测试中抵达终层的中间楼梯命令 | 128 次 | 9 次，另有 9 次直接准备守关前一层 |
| Warrens 全九层往返 | 16 个种子 | 1 个种子（2） |
| Warrens 地图结构／确定性／保存检查 | 16 个种子 | 3 个种子（0、1、2） |
| 自然词缀掉落 | 20,000 次连续生成 | 8 个独立命名种子 |
| 按深度共享掉落等价 | 256 个种子 × 5 次生成 | 4 个种子 × 5 次生成 |
| 上一行的世界初始化 | 1,281 次 | 1 次，复用其克隆 |
| 矿坑三种河流结果 | 搜索上限 128，本次实际 15 个种子 | 3 个明确分支种子 |

世界模块同机本轮聚焦运行：**66 项／35.08 秒 → 65 项／9.48 秒**，均全通过。不是多轮性能基准，不能将该比例外推到整个 Core。

## 代表层与明确减少的深度覆盖

保留入口及尺寸边界、守关前层／终层，以及原测试明确检查的湖泊、破坏、竞技场、帘幕、玻璃等特殊层。每层仍检查原来的地形、连接、楼梯及相应守卫结果。全定义数量断言继续保留；它不是被省略楼层的生成验证。

| 地牢 ID 后缀 | 继续生成的深度 | 不再在该布局测试中生成的深度 |
|---|---|---|
| `numenor` | 55, 56, 65, 70, 74, 75 | 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 71, 72, 73 |
| `atlantis` | 55, 60, 64, 65 | 56, 57, 58, 59, 61, 62, 63 |
| `giants-hall` | 30, 31, 39, 40 | 32, 33, 34, 35, 36, 37, 38 |
| `snow-castle` | 30, 31, 49, 50 | 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48 |
| `graveyard` | 50, 51, 54, 62, 66, 69, 70 | 52, 53, 55, 56, 57, 58, 59, 60, 61, 63, 64, 65, 67, 68 |
| `witch-wood` | 25, 26, 39, 40 | 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38 |
| `plains-of-oz` | 18, 19, 35, 36 | 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34 |
| `troll-cave` | 18, 19, 24, 30, 31, 35, 36 | 20, 21, 22, 23, 25, 26, 27, 28, 29, 32, 33, 34 |
| `eyrie` | 40, 41, 49, 50 | 42, 43, 44, 45, 46, 47, 48 |
| `castle` | 40, 41, 45, 50, 55, 64, 65 | 42, 43, 44, 46, 47, 48, 49, 51, 52, 53, 54, 56, 57, 58, 59, 60, 61, 62, 63 |
| `icky-cave` | 10, 11, 19, 20 | 12, 13, 14, 15, 16, 17, 18 |
| `tidal-cave` | 15, 16, 26, 27 | 17, 18, 19, 20, 21, 22, 23, 24, 25 |
| `lonely-mountain` | 30, 32, 34, 35, 38, 39, 40 | 31, 33, 36, 37 |
| `dragon-lair` | 60, 62, 64, 66, 68, 71, 72 | 61, 63, 65, 67, 69, 70 |

`p92c_labyrinth_generation_keeps_the_small_perfect_maze_connected` 保留全部九层及迷宫连通性、557 个地板格、合法怪物落点和最终守卫检查。固定任务地图、户外滚动、埋伏、变色龙、火山及其他独特算法测试保留。

两个选样修正有明确原因：Eyrie 的普通四层最初没抽到可选河流，因此用已验证的 RNG 种子 3 开始生成，保留水域正例；Troll Cave 的总泥土地形预算按所选层数计算，仍保持平均每层超过 240 的原约束。Warrens 选用种子 2，它在完整旅程中同时产生超出最低数量的掉落和深度门槛物品；没有删除相应断言。

这些改变减少了未列出深度和种子的组合检测，也改变了部分代表层到达时的 RNG 前置状态；不宣称与原全层、长序列抽样等价。

## 逐项替换与行为归属

### 布局测试

| 原函数 | 当前函数 |
|---|---|
| `p96d_all_numenor_atlantis_floors_generate_water_veins_and_ordinary_stairs` | `p96d_representative_numenor_atlantis_floors_generate_water_veins_and_ordinary_stairs` |
| `p99f_all_giants_hall_and_snow_castle_floors_generate_without_doors` | `p99f_representative_giants_hall_and_snow_castle_floors_generate_without_doors` |
| `p100f_all_graveyard_floors_generate_shallow_water_layers_and_shafts` | `p100f_representative_graveyard_floors_generate_shallow_water_layers_and_shafts` |
| `p101d_all_witch_wood_and_plains_of_oz_floors_generate_their_outdoor_layers` | `p101d_representative_witch_wood_and_plains_of_oz_floors_generate_their_outdoor_layers` |
| `p90c_troll_cave_generation_keeps_terrain_mix_lakes_shafts_and_connectivity` | `p90c_troll_cave_generation_keeps_terrain_mix_lakes_shafts_and_connectivity` |
| `p91c_eyrie_generation_keeps_caverns_rivers_shafts_and_connectivity` | `p91c_eyrie_generation_keeps_caverns_rivers_shafts_and_connectivity` |
| `p98c_castle_generation_keeps_rooms_stairs_and_representative_layers` | `p98c_castle_generation_keeps_rooms_stairs_and_representative_layers` |
| `p88e_icky_cave_all_depths_keep_the_terrain_mix_and_stairs_reachable` | `p88e_icky_cave_representative_depths_keep_the_terrain_mix_and_stairs_reachable` |
| `p87e_tidal_cave_all_depths_keep_water_and_stairs_reachable` | `p87e_tidal_cave_representative_depths_keep_water_and_stairs_reachable` |
| `p93c_lonely_mountain_generation_keeps_lava_caverns_lakes_and_destruction` | `lava_cavern_dungeons_share_lakes_destruction_stairs_and_guardian_placement` |
| `p97e_dragon_lair_generation_keeps_lava_caverns_lakes_and_guardians` | `lava_cavern_dungeons_share_lakes_destruction_stairs_and_guardian_placement` |

孤山和龙穴共享 `lava_cavern_dungeons_share_lakes_destruction_stairs_and_guardian_placement` 的两个命名地牢案例。仍分别检查熔岩、树湖、碎石、普通楼梯、可走的怪物落点，以及 Smaug／Tiamat 对应的守卫 ID 和类型。

### 征服准备

`enter_guardian_floor_from_penultimate` 调用既有 `transition_floor` 准备守关前一层，再执行真实的最后一级普通下楼命令；没有手工拼装存档状态。以下测试原有入场、入口守卫、竖井跳层、遗忘、击杀、一次性奖励、替代奖励、激活、禁附魔与保存断言均留在各自函数内。

| 保留的测试 | 原中间下楼目标 | 现在准备／最后一级楼梯 |
|---|---|---|
| `p90c_troll_cave_shared_entry_shafts_conquest_and_reward_are_one_shot` | 19–36 | 35 → 36 |
| `p91c_eyrie_guardians_shafts_conquest_and_new_life_reward_are_one_shot` | 41–50 | 49 → 50 |
| `p92c_labyrinth_forgets_after_movement_and_drops_the_fixed_recall_rod` | 21–28 | 27 → 28 |
| `p93c_smaug_drops_arkenstone_with_clairvoyance_and_replacement` | 31–40 | 39 → 40 |
| `p97e_dragon_lair_guardians_and_scale_mail_reward_are_one_shot` | 61–72 | 71 → 72 |
| `p98c_castle_guardians_and_conquest_are_one_shot` | 41–65 | 64 → 65 |
| `p100f_graveyard_guardians_and_rolled_soulsword_reward_are_one_shot` | 51–70 | 69 → 70 |
| `p94c_mine_guardians_and_star_healing_reward_are_one_shot` | 76–80 | 79 → 80 |
| `p95c_battlefield_guardians_reward_and_no_enchant_are_one_shot` | 31–50 | 49 → 50 |

这里明确放弃的是中间每一级楼梯的独立运行抽样，以及相应赶路产生的随机序列；没有把它们记作仍被验证。Warrens 全深度往返、Orc Cave 征服后回地表，以及 Camelot／Tidal Cave／Icky Cave 的召回、往返和实例生命周期集成继续保留。

### 掉落

- `base_item_natural_egos_cover_completed_weapon_digger_and_ranged_types` 保留原真实 loot 入口、质量、词缀数量、rolled 字段一致性、类型限制、保护防御范围和不兼容降级检查。种子 `1, 9, 44, 114, 248, 305, 1124, 9477` 分别锁定弹药、普通优良护甲、发射器、武器、保护、挖掘工具、竖琴和 Buckland 投石索；新增对应物品／词缀身份断言，避免样本失去预定分支而不自知。
- 新 loot 样本没有 164／165 两个受限发射器词缀的正例；原随机测试只在遇到它们时检查 subtype。既有 `ego::tests::restricted_launcher_egos_retry_without_partial_rolls` 继续保留 164／165／166 的正负类型、拒绝原子性及精确 RNG 检查。它覆盖词缀实例化，不等于保留其他 loot 种子的接线抽样。
- `shared_base_and_warrior_loot_use_depth_instead_of_dungeon_identity` 用种子 `0, 1, 42, 255` 检查深度 9 候选、深度 15 和 20 的跨地牢掉落等价；每行仍走实际生成，只减少世界初始化和其他种子。
- `p94c_mine_generation_selects_dry_water_or_lava_rivers_with_rich_veins` 的种子 `0/5/14` 明确对应无河／熔岩河／水河，继续检查互斥、地图尺寸和富矿脉。未保留搜索途中的其他结果抽样，不宣称检验概率分布。

### 设施与任务状态

| 原函数 | 当前函数／承接检查 |
|---|---|
| `anambar_inn_stay_advances_half_day_and_restores_the_player` | `inn_stays_use_content_prices_and_restore_the_player_at_half_day` |
| `white_horse_inn_uses_its_content_price` | 同上；任务服务投影迁入 `trouble_at_home_runs_from_white_horse_targets_only_mercenaries_and_rewards_warrior` |
| `count_grants_the_warrior_broad_sword_only_when_claimed` | `count_task_rewards_complete_only_on_claim_with_the_expected_inventory_item` |
| `count_grants_the_fur_cloak_only_when_pest_control_is_claimed` | 同上，两项任务各一行 |
| `clearing_thieves_hideout_closes_the_floor_without_granting_the_reward` | `thieves_hideout_departure_closes_the_entry_and_keeps_reward_and_failure_distinct` |
| `leaving_thieves_hideout_uncleared_fails_and_closes_the_entry` | 同上，完成／未完成各一行 |

旅店保留 25／20 收费、75／0 余额、半日时间、恢复／状态清理、充能、营养、RNG 和保存检查；白马服务仍检查入口及首个任务。旅店拒绝测试只将三次初始化改为一次后克隆，毒／流血及余额不足分支都保留。

领奖测试保留 prerequisite、Completed 状态、奖励实例／物品／库存位置、无 RNG；Pest Control 行也增加无 RNG 断言。离开盗贼巢穴继续区分 RewardAvailable 与 Failed、入口地形、回地表和未提前发奖；成功行仍检查奖励可领取事件。原失败案例出生种子 44 统一为 43；Pest Control 奖励案例 55 统一为 45，减少的是无关世界初始化变化。

商店交易拒绝、库存实例、收费原子性、任务先决条件、任务独特目标筛选、魔法楼梯、被阻挡楼层清理及退休终态等测试保留。

## 验证

- 新增或修改的代表层、固定种子、领奖、任务离开和旅店案例，以及迁入白马服务投影的原任务流程，均通过聚焦测试。
- 世界模块：65 项全部通过；`cargo fmt --all` 和改动源码的 `git diff --check` 通过。
- `cargo test -p rfb-core --lib`：**871 项全部通过**，0 失败／忽略，68.20 秒。第二步结束时为 875 项、96.28 秒；这是两次普通运行的观测值，不是受控性能基准。
- 本轮只修改上述三个测试文件和执行记录；没有修改生产代码、Content 包、协议、状态哈希输入或回放 fixture。未运行 Content 全套、桌面 E2E、verify-all 或 refresh-all。
