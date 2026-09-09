> 历史快照（2026-09-09 归档）：本文保留当时的设计、版本与验收记录，不作为当前工作指令或待办。现行说明见 [文档索引](../../../README.md)。

# rfb-core 测试缩减：第五步执行记录

日期：2026-09-09。基线为[第四步](core-test-reduction-stage4.md)完成后的 867 项。范围为投影、存档字段、序列化冒烟和重复规则检查。

## 实际结果

Core **867 → 863 项**，八个源码文件中的测试代码合计净减 **23 行**。四个必填字段测试合并为一个七行参数矩阵；一个独立特征详情保存冒烟测试并入已有光环测试。没有新增 ignored 测试，没有迁移到其他 crate，没有改变生产代码或存档格式。

≤300 项预算未达到。延续用户“没有其他能删的就不删”的要求，保留尚无完整替代的行为检查。完整快照相等只能替代其中的字段相等，不能替代固定期望值、拒绝、知识隐藏或运行时规则测试。

| 文件 | 测试数 | 源码行数变化 |
|---|---:|---:|
| [game/tests/persistence.rs](../../../../crates/rfb-core/src/game/tests/persistence.rs) | 8 → 9 | +38 |
| [game/tests/riding.rs](../../../../crates/rfb-core/src/game/tests/riding.rs) | 11 → 10 | -10 |
| [game/tests/weapon_proficiency.rs](../../../../crates/rfb-core/src/game/tests/weapon_proficiency.rs) | 8 → 7 | -11 |
| [game/tests/mining_progress.rs](../../../../crates/rfb-core/src/game/tests/mining_progress.rs) | 7 → 6 | -13 |
| [game/tests/monster_ecology.rs](../../../../crates/rfb-core/src/game/tests/monster_ecology.rs) | 36 → 35 | -11 |
| [game/tests/progression.rs](../../../../crates/rfb-core/src/game/tests/progression.rs) | 41 → 41 | -17 |
| [game/tests/trait_details.rs](../../../../crates/rfb-core/src/game/tests/trait_details.rs) | 9 → 8 | -4 |
| [game/mogaminator.rs](../../../../crates/rfb-core/src/game/mogaminator.rs)，仅测试模块 | 10 → 10 | +5 |

## 必填字段矩阵

新增 `game::tests::persistence::new_save_required_fields_reject_missing_values`。共享一个合法新存档，每行只删除一个字段，并检查错误明确指出该缺失字段。先确认原存档能解码、待删字段存在，避免其他无关错误使负例意外通过。

| 原测试或检查块 | 保留字段 |
|---|---|
| `game::tests::riding::riding_proficiency_save_field_is_required` | `/player/progress/ridingProficiency` |
| `game::tests::weapon_proficiency::weapon_proficiency_save_field_is_required_for_new_progress_payloads` | `/player/progress/weaponProficiencies` |
| `game::tests::mining_progress::mining_and_material_save_fields_are_required` | `/player/progress/miningProficiency`、`/player/progress/materials` |
| `game::tests::monster_ecology::defeated_limited_actor_counts_are_required_in_new_saves` | `/defeatedLimitedActorCounts` |
| `game::tests::progression::attribute_and_experience_history_round_trip_and_reject_invalid_values` 中的缺失字段循环 | `/player/progress/maximumAttributes`、`/player/progress/maximumExperience` |

上述四个独立测试原来共创建并编码 5 次新存档，现为 1 次。属性／经验历史测试仍保留其非默认值的 JSON 编解码、完整状态哈希和非法值拒绝，只将必填字段检查移入矩阵。

缺失字段统一使用种子 `0x5354_5249_4354` 的初始值；不再分别使用骑乘的种子 48、怪物计数的种子 18，以及属性／经验历史改变后的载荷做字段缺失检查。所有字段行均保留；不是保留原来的全部种子和载荷组合，也没有添加旧开发存档兼容。

## 完整结果与字段去重

在 `game::tests::progression` 的以下三个测试中保留完整快照相等，移除合计五条重复断言：

| 测试 | 移除的重复检查 |
|---|---|
| `build_skill_growth_experience_multiplier_and_save_identity_are_deterministic` | 恢复后的 `build`、`progress.skills` 再次与原值比较。 |
| `high_elf_intrinsics_and_identity_round_trip` | 恢复后的 `build` 和 `state_hash()` 再次与原值比较。 |
| `selected_formal_race_overrides_the_build_default_and_round_trips` | 恢复后的 `build` 再次与原值比较。 |

依据是 `Game::snapshot()` 包含构建／技能投影及 `state_hash()`，后者的输入包含保存的玩家身份和技能状态。固定成长数值、明确种族 ID 和非法构建拒绝等期望断言继续保留，没有用“两边相等”替代正确性检查。

删除 `game::tests::trait_details::trait_details_are_stable_across_save_round_trip` 独立冒烟入口。其 `STATUS_REGENERATION`、`STATUS_HOLD_LIFE` 状态和详情恢复相等保障转入 `trait_details_auras_share_combat_sources_without_rolling_damage`，后者继续检查光环来源、投影不改变存档／RNG、状态移除，并新增完整快照往返相等。

该合并使用已有光环测试的种子 42／清空物品的 Warrior，而非旧冒烟的种子 0／出生物品。直接构造的六个状态按 ID 排序，与正常状态应用和存档恢复的顺序一致。旧的未排序两状态载荷不再单独抽样；原状态内容的恢复保障保留，并扩展到光环及完整快照。

特征详情的未知装备隐藏、抵抗合并、来源计数、单位、攻击范围和定向感知测试继续独立。地面物品的可见／探测投影、状态哈希排除项、内容不匹配、损坏存档拒绝及非默认状态恢复测试均保留。

## 规则扫描与辅助函数

`game::mogaminator::tests::bilingual_defaults_match_every_current_item_kind_equally` 仍遍历所有当前物品、所有规则及六种知识组合，分别比较中英文名称搜索结果、动作、题记、条件激活和最终匹配结果。

仅在两边谓词向量完全相同时复用英文侧已算出的谓词结果；向量不同时仍分别计算。没有断言中英文 AST 必须相等，没有减少语言、规则、物品或知识组合。聚焦测试本轮 **11.38 → 11.02 秒**，只是各一次运行，未体现显著提速。

检查了本步删除测试涉及的辅助函数和引用：四个必填字段测试无专属辅助函数；特征详情的 `game`、`equip`、`status`、`details` 均仍有实际调用。本步没有新增辅助函数，也没有发现因本步删除而失去用途的辅助代码。

## 验证

- 七个必填字段行、合并后的光环／保存测试、完整双语规则扫描均通过聚焦测试。
- `cargo test -p rfb-core --lib`：**863 passed，0 failed，0 ignored**，61.91 秒，无编译警告。
- `cargo fmt --all -- --check`、`git diff --check`：通过；Git 仅报告工作区既有文件的换行转换提示。
- 未修改内容包、状态哈希输入或协议，未刷新 fixture，未运行桌面 E2E。
