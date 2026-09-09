# 地牢城镇首批：地点来源修正与竞技场计划

工作树：`D:/codex/RoguelikeFansBand-Rewrite-dungeons-towns`；分支：`codex/dungeons-towns`。
起始 main：`62f959f3bb6b308205cd1fb53ae9c8a99f59b391`。接手时工作树已存在且干净，直接复用。
权威 RFB `master`：`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`；以下资料均通过 Git 对象读取。

## 本批实际改动

- 修正 `demo.dungeon.crystal-castle` 的世界坐标：原版 `lib/edit/d_info.txt` 的 `N:20`、`P:40:37` 按 `(y,x)` 解析，对应 `(x=37,y=40)`。原计划、正式世界位置和两项内容断言误写为 `(40,37)`，本批一起修正。
- 修正水晶城堡局部入口守卫位置：原 `(48,16)` 与玩家离开世界地图后的落点重合，`spawn_visible_dungeon_entrance_guardians` 因占位而跳过守卫生成。将内容中的守卫锚点移到相邻空位 `(49,16)`，复用现有生成规则；此局部锚点是 Rewrite 地图适配，不是原版世界坐标。
- 修正地点来源校验：无固定奖励的 `Castle` 是合法来源记录。校验器原来把“普通奖励与神器奖励均为空”误判为错误；现在只拒绝同时配置两类固定奖励，继续逐项核对原版守卫、奖励、Ego 与替代地牢。
- 按 [地牢交接的 A 阶段](dungeon-addition-handoff.md#41-a锁定原版计划) 增加竞技场来源计划与聚焦测试，尚未将其加入正式地点。

水晶城堡的内容、入口已修正；规则复用现有地牢与荒野路径。竞技场仅完成来源计划，未定义正式楼层、未开放入口、未完成玩家流程验收。

## 竞技场原版事实及依赖

| 项目 | 来源与结论 |
| --- | --- |
| 地牢身份 | `lib/edit/d_info.txt N:25:Arena`，稳定 ID `demo.dungeon.arena`；`src/dungeon_name_zh.inc` 的中文名为“竞技场” |
| 坐标、深度 | `P:7:67` → `(67,7)`；50–80 层 |
| 表内生成参数 | `NO_VAULT | BIG`；无 monster preference；无显式 `MONSTER_DIV`，现有解析值为 0；地面比例 `FLOOR 100/0/0`，tunnelPercent 8 |
| 入口守卫 | `lib/edit/r_info.txt N:691:Drolem`，48 级；中文表索引 691“龙魔像”；复用 `demo.actor.drolem` |
| 最终守卫 | `N:1110:Metal Babble`，80 级；中文表索引 1110“散失金属史莱姆”；复用 `demo.actor.metal-babble-unique`，不能误用非唯一 `demo.actor.metal-babble` |
| 固定奖励 | `FINAL_OBJECT_70_52`；`lib/edit/k_info.txt` 的 `Artifact Creation:engsno argsan`；无固定 Ego、神器或替代地牢 |

正式开放前仍需完成：

1. **地牢城镇负责生成与生态。** `src/rooms.c room_build/build_type16(TRUE)` 使用半径 3–7 的圆形房间及中心单怪；`src/generate.c _cave_gen_monsters/_cave_gen_objects` 排除普通追加怪物和随机物品；`src/monster2.c get_mon_num` 对非召唤选择设置 `MIN(50, level - 5)` 的最低等级，`place_monster_aux/alloc_monster` 排除同伴、护卫和怪物群。现有 `ProceduralRoomShape` 仅支持 rectangle/cross/cavern；不能仅复制普通地牢并加 `BIG` 就视为实现。本轮不增加共享 DTO 或 RNG 路径，后续应独立交付必要的公共底座。
2. **法术道具负责最终奖励。** 所需来源身份为 `(tval=70,sval=52)`，正式物品 ID 尚未提供，依赖提交待定。现有 `legacy-item-p3-plan.json` 的 `artifact-creation` 仍标记 `random-artifact-identity` 阻塞。需要真实随机神器生成、合法目标与失败/取消语义、保存恢复；中文物品名待该方向从权威表核定（本批 unresolved）。`demo.item.crafting-scroll` 是另一种卷轴，不能替代奖励。
3. **地牢城镇负责正式闭环。** 以上依赖完成后再增加局部荒野入口、连续楼层、奖励表，验证守卫唯一结算、返回/召回及存档恢复。原版守卫 actor 已有定义，本批未重新导入或宣称已逐项验收其能力。

## 集成影响

- 本批更改 importer、正式世界坐标、来源计划及对应测试；共享冲突文件为 `crates/rfb-legacy-import/src/content.rs`、`worlds/middle-earth.json`、`legacy-wilderness-selection.json`、`pack.json`、`content.lock.json`、`design/current-status.md`。
- 内容包 `1.384.1`，内容数量不变。升版和更新 lock 的原因是水晶城堡正式世界坐标与入口守卫锚点变化；竞技场计划本身不进入编译内容。
- 协议仍为 `1.230`，save 仍为 v5，State Hash Schema 仍为 v108；未改变状态结构、公共初始化或 RNG 算法/调用顺序。正确坐标的局部荒野使用该坐标派生的既有生成路径。
- 26 条 active exact fixture 均不含水晶城堡或世界地图进入命令，本批不刷新 fixture、不运行全量回放。仅涉及内容与 importer，不需要重新生成协议绑定或内容 Schema。
- 在主线按交接流程合并本提交后，重新计算集成后的内容版本与 lock；不要沿用其他方向的过期 hash。未合并或推送 main。

## 验证

以下命令均在本方向工作树执行。来源同步输出写到 `.tmp/arena-source-world.json`，不覆盖正式世界中的手工接入。

```powershell
cargo test -p rfb-legacy-import --lib _plan_
$env:RFB_LEGACY_SOURCE = 'D:/codex/Frogcomposband/master'
New-Item -ItemType Directory -Force .tmp | Out-Null
Copy-Item packs/rfb-demo-original/worlds/middle-earth.json .tmp/arena-source-world.json
cargo run -q -p rfb-legacy-import -- sync-demo-wilderness packs/rfb-demo-original/legacy-wilderness-selection.json .tmp/arena-source-world.json
cargo test -p rfb-content --lib special_layout_dungeon_bindings_match_source
cargo test -p rfb-content --lib town_entrances_and_shared_facilities_match_source
cargo test -p rfb-core --lib crystal_castle_entrance_
cargo run -q -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -q -p rfb-contract -- verify tests/fixtures/active/scenarios/19-save-round-trip-initial.json
cargo fmt --all -- --check
git diff --check
```

结果：26 项 importer 计划测试、2 项相关内容测试、1 项水晶城堡核心闭环测试全部通过；完整地点来源同步通过（输出 18 个所选正式地点，保留在临时输出中）；内容锁、初始存档 exact fixture、格式和 diff 检查通过。初始 fixture 无需刷新，确认本批没有改变其状态哈希。

未运行完整桌面/E2E、Android、全量回放；未进行人工试玩。
