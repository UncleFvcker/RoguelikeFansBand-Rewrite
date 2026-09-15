# 通用操作第一步：命令／键位对照与连续行动取消

状态：2026-09-13，K1 静态审计、K2–K4 实现与定向自动验证已完成；K5 待实施。尚未经 K5 的原生桌面实按验收，不代表完整原版键位兼容。

## 目标与范围

完成一份以实际源码为依据的完整原版命令／键位对照，并让当前已有的连续行动能够在完整行动边界停止。执行次序为 K1 → K2 → K3 → K4 → K5。

本批的“完整”指审计覆盖全部原版命令及默认键位映射，不指实现全部原版功能。对照必须区分规则存在、界面入口存在、快捷键存在和实际验收；已有按钮不能直接算快捷键完成。

实施范围包含局部旅行、世界地图旅行、自动拾取和恢复休息的主动取消，并统一钓鱼与现有方向选择的中止边界。保留现行 numpad／vi／WASD 预设；仅调整本批取消入口及审计确认会误触动作的输入冲突。

完整 Original／Roguelike 预设的实际接入、自动探索、跑步、数字计数、重复上次命令、持续搜索、扩充休息模式、宏、宠物高级指令和知识菜单列入对照的后续项。本批不扩充种族职业、法术道具、地牢城镇内容。

## 基线与来源

- 工作树：`D:/codex/RoguelikeFansBand-Rewrite`，`main@73e760297da43d4ef9c9546a697b4e51ce6c3e00`。用户已接管 main；实施前重新核对 HEAD 和实际修改，保留未跟踪的 `release/`。
- 原版仓库：`D:/codex/Frogcomposband/master`，本次实际来源 `master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。使用 `git show`／`git grep` 读取 Git 对象；后续 master 变化时记录差异，不静默混用来源。
- 原版命令依据：`src/dungeon.c` 的实际分发，`src/util.c::request_command` 的输入处理，`lib/pref/pref-key.prf` 及其实际加载的默认／平台映射；根据调用关系核对 `cmd1.c`、`cmd2.c`、`cmd4.c`、`cmd5.c` 等消费者。
- `lib/help/command.txt`、`commdesc.txt` 用于辅助说明。当前 `Z` 的帮助描述与 `dungeon.c` 实际自动探索分发不同，发生冲突时记明源码行为，不照抄帮助表。
- 当前入口：[InputController](../web/src/input-controller.ts)、[GameSession](../web/src/game-session.ts)、[main](../web/src/main.ts)、[状态面板](../web/src/status-panel.ts)、[设置](../web/src/settings-panel.ts)。
- 当前规则：[局部旅行](../crates/rfb-core/src/game/travel.rs)、[荒野旅行](../crates/rfb-core/src/game/wilderness.rs)、[自动取物](../crates/rfb-core/src/game/mogaminator.rs)、[休息](../crates/rfb-core/src/game/player_abilities.rs)、[命令结算](../crates/rfb-core/src/game/mod.rs)。

复查修正：当前挖掘失败后只是重新进入方向选择，并非自动持续发出挖掘命令。本批保证这个待输入状态可取消，不为它新增自动挖掘循环。

## K1：完整命令与有效键位对照

**已完成：**见[原版命令、默认键位与当前入口对照](command-keymap-audit.md)。原版分发及默认映射均有去向，连续行动调用者已列明。已确认当前 i/I 背包和 m/M 能力页入口；WASD c 被关门截走、修饰键穿透等差异见对照表 F1–F9，留待 K4 修复。本批仅修改文档。

交付 `docs/command-keymap-audit.md`，作为本批唯一的对照清单。按用户命令逐项记录，别名合并但不能漏列；不新增审计框架或靠数量断言凑测试。

每项记录以下信息：

| 字段 | 要求 |
| --- | --- |
| 原版命令和来源 | 源函数、分发分支、来源提交及行号 |
| 原版有效按键 | Original／Roguelike 两套；区分物理按键、映射展开后的命令、Ctrl／Shift、平台差异 |
| 交互上下文 | 地图、方向选择、目标选择、物品／法术选择、对话框；前置状态和取消含义 |
| 当前规则与入口 | 生产 GameCommand／核心消费者、按钮或面板、各预设快捷键；无消费者则明确缺失 |
| 差异分类 | 已对齐、功能存在但缺键、键义不同、规则缺失、平台适配、开发专用 |
| 后续归属和证据 | 本批取消／冲突修复、后续通用系统、其他方向、明确不移植的终端操作及理由；关联有效测试 |

覆盖移动、停留、跑步、旅行、探索、取物、搜索、休息、地形交互、目标与观察、物品／能力菜单入口、宠物、角色／知识／消息、保存退出、设置、宏录制及特殊命令。种族职业和物品具体效果不在本批逐项展开，但其通用操作入口必须入表。

同时核对当前大小写折叠和修饰键穿透：例如 `r/R`、`S`、`v/V`、`J`、vi 方向、装置菜单和 Ctrl 组合。不能把 Ctrl 组合误当普通字母动作；IME 合成、文本输入、原生按钮 Enter／Space 和对话框自有按键保持各自行为。原版有自杀等特殊映射，记录其存在不等于未经确认直接接入。

**完成条件：**原版分发分支及两套默认映射都有去向；所有现有连续行动都有真实调用者清单；未核对项明确保留，不能写成“全部键位支持”。

## K2：统一已有旅行与拾取的取消

**已实现并通过定向自动验证。** `InputController` 的局部旅行（含物品列表调用）、世界旅行、自动取物共用一次活动循环；`GameSession.dispatch` 明确返回 `applied`／`blocked`／`failed`，每步让出一个事件任务。Escape、游戏按键和地图点击消费本次取消输入；单独修饰键、系统快捷键、编辑／合成输入不误触循环内动作。停止按钮忙碌时可用，重复取消不提交命令，局部／世界目的地仍可用 J 手动继续。

原生创建／覆盖／加载存档、导出／导入、菜单／新游戏和关闭窗口已接等待当前循环结算的边界；存储期间沿用游戏 busy 锁。失焦、隐藏、销毁与重置清理续发任务。已有危险／无进展判断保留，派发失败或待选择直接停止。此处只处理三类 K2 循环；钓鱼的必要 CancelFishing、付费瞄准和其余旧回调收口仍属 K4。

验证：`web` 下执行 `node --test src/input-controller.test.ts src/game-session.test.ts src/save-panel.test.ts src/app-dom.test.ts src/localization.test.ts src/object-list.test.ts src/player-ui-layout.test.ts`，62 项通过、0 跳过；`npm run typecheck` 通过。新增用例使用真实 InputController／GameSession、可控制完成时机的 Core 响应和任务计时器，覆盖在途及两步间取消、取物外层换目标、J 恢复、错误／拒绝／选择、危险停止、失焦／隐藏／销毁／重置、地图点击，以及原生保存等待最新状态和锁定。它们不替代真实 Rust 移动或桌面实按验收。没有更改 Rust 规则、协议、内容包、存档格式或哈希，也没有刷新 fixture；K5 再统一交付 standalone 与桌面证据。

直接使用 GameSession 的决斗者／食魔者消费者另执行 `node --test src/duelist-panel.test.ts src/magic-eater-panel.test.ts`，9 项通过；合计 71 项定向测试通过、0 跳过。

在现有输入控制职责内管理一次连续行动，只保存当前循环种类、取消状态及区分旧循环所需的最小标识。局部／世界旅行沿用既有目的地；不建立持久任务系统、命令队列或新的寻路规则。

### 取消语义

| 触发 | 行为 |
| --- | --- |
| Escape／明确的“停止”按钮 | 即使请求尚在执行也能登记取消；当前已发送的核心动作完整结算，之后不再发送下一步 |
| 连续行动期间的另一游戏按键／地图点击 | 本次输入仅停止，不同时移动、施法或设新目标；用户再次输入才执行新动作。修饰键单独按下和系统快捷键不作为游戏命令 |
| 保存、菜单、加载、新游戏、退出 | 先停止继续发命令，等待当前动作及必要取消动作结算；保存使用结算后的状态。旧异步回调不得操作新会话 |
| 失焦／页面隐藏／控制器销毁 | 停止续发并清理定时器；重新聚焦不自动继续旅行、拾取或休息 |
| 错误、请求被拒绝或核心要求选择 | 停止循环并保留真实提示，不能重试同一动作形成忙循环 |
| 敌人、受伤、阻路等已有停止条件 | 保持现有条件；本批只补取消和停止原因，不在前端增加游戏规则 |

每次最多一个生产请求在途。取消监听在普通 busy 拦截之前运行，每步结束给事件循环处理输入的机会；不以持续微任务循环或并发请求追求速度。停止按钮要在 busy 时可用，但其他动作仍遵守现有锁。

K1 基线的 `GameSession.dispatch()` 捕获异常并返回 `void`；K2 已改为直接返回成功更新、拒绝执行或失败的结果，所有命令仍经同一 GameSession 派发。

局部旅行取消后可保留同层目的地供 `J` 手动继续；世界旅行沿用现有核心目的地。切换地图／角色时按现有坐标和楼层资格清理。取消本身不撤销 RNG、移动、伤害或拾取，不发送额外 Wait，不增加游戏时间。

**完成条件：**局部旅行、世界旅行和自动拾取在取消后至多完成一条已在途动作；没有后续移动／拾物请求；重复取消不产生动作。取消触发的输入不会落到后面的游戏界面执行。

## K3：让现有恢复休息可取消

**已实现并通过定向自动验证。** `r/R` 与状态面板按钮统一调用 `InputController.restUntilRecovered()`，逐条发送 `Rest { turns: 1 }`，沿用 K2 的串行、取消和保存等待边界。只在核心 Rest 回执为“完成 1 回合、TurnLimit”时继续，累计预算 9999；其他停止原因、错误、拒绝和待选择均不续发。核心命令／回放保留全部回执，消息面板省略单步 TurnLimit 的完成提示，最终停止和预算耗尽仍可见。合法读档只恢复核心状态，不自动重启休息循环。

修复唯一的规则计数差异：`game/mod.rs` 不再把 0 个完成回合强制计为 1 turn；已满资源／无法开始的休息保持原 world_tick 与 RNG，并保持 turn 不变。命令序号和 revision 仍正常递增；原有启动休息清除狙击专注的语义保留。Rust 的恢复、伤害、召回、宠物、状态／装置 tick 和按 completed_turns 累计的能力冷却逻辑未搬到前端或另建计算路径。

批量／单步对照以相同起点验证 HP／MP、加速状态与饥饿时钟、食魔者体内 SP 和小数进度、宠物维持、满 HP 召回。仅统一 revision／command_seq 后，完整保存数据和 RNG 相同；每一步另从存档恢复，核对相同下一条命令的完整 GameUpdate 与状态哈希。决斗者专项验证休息中出现传送选择时当前回合先挂起，保存后确认只结算一次。ReplayRecorder 专项覆盖单步休息、中途保存恢复、随后休息和手动 Wait 的编码／解码／验证，完整事件和哈希一致。等价性证据针对这些场景，不把不同命令流的整体哈希或所有动态地图场景笼统宣称等价。

验证命令与结果（仓库根目录执行 Rust，`web` 执行前端）：

```powershell
cargo test -p rfb-core --lib -- game::tests::abilities::restoration:: game::tests::hunger:: game::tests::pet_upkeep:: game::tests::magic_eater::usage:: game::tests::duelist::choices:: game::tests::mindcrafter:: game::tests::sniper:: game::tests::waste::
cargo test -p rfb-replay --lib tests::single_step_rest_save_resume_replays_identical_events_and_hashes -- --exact
cargo run -p rfb-contract -- verify-all tests/fixtures/active/baseline-policy.json
node --test src/input-controller.test.ts src/status-panel.test.ts src/message-panel.test.ts src/game-session.test.ts src/save-panel.test.ts src/localization.test.ts
npm run typecheck
```

核心 132 项、回放 1 项、前端 66 项通过，均无跳过；26 条现有契约原样通过，未刷新 fixture。`cargo clippy -p rfb-core -p rfb-replay --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和文档检查通过。前端覆盖键盘／按钮共用入口、在途取消、所有终态与 9999 总预算、中间消息抑制及既有 K2 保存／取消回归。没有修改内容包、协议或保存／哈希 Schema；桌面实按与 standalone 仍在 K5 交付。

以下保留本批设计与完成条件。K2 基线的按钮及 `r` 都发送 `Rest { turns: 9999 }`，核心在一次请求中循环，前端停止标志无法中断它。本批保留“恢复资源并等待召回”的现有休息目标，不同时实现原版数字、`*`、`&` 多模式。

UI 改为逐次请求现有 `Rest { turns: 1 }`，复用 K2 的串行执行与取消；键盘和状态面板必须调用同一入口。是否继续只消费核心返回的 `RestResolutionDto`：完成一轮且原因是 `TurnLimit` 才续发，资源已满、受伤、可见敌人、死亡或待选择等终态立即停止。达到现行 9999 次预算也停止，不能把预算在每次请求重置。

这不是简单把 9999 改成 1 就算完成。重点核对核心按命令与按世界 tick 的结算差异：

- HP／MP／体内装置恢复、饥饿、异常状态、召回、宠物费用和能力冷却。
- `game/mod.rs` 的回合推进、命令后可见性／事件处理，以及决斗等待选择后的继续结算。
- 零个实际休息回合是否仍推进了 turn；满资源后不能因循环多发空休息。

以相同起始状态对照旧批量休息与单步休息的实际玩法结果和后续 RNG；命令序号／revision／逐条事件封装因请求数量不同而不同，不能要求不同命令流的完整哈希相等，也不能借此忽略规则差异。发现错误的按命令结算时，在现有休息消费者中修正并说明影响；不在前端模拟恢复、冷却或随机数。不允许带着未解释的结算差异交付。

UI 的单步命令序列必须能够经现有保存／回放重现。保存后恢复相同命令流，应获得相同事件和完整哈希；普通休息循环不自动随读档重启。

**完成条件：**按键和按钮启动的休息均能停止；满 HP 等待召回仍有效；中断后的下一次操作正常；已提交回合完整保留，不丢失或重复结算。

## K4：输入上下文、钓鱼和生命周期收口

**已实现并通过定向自动验证。** 钓鱼复用连续行动状态、计时让步、Escape／停止按钮和失焦／隐藏取消。取消先等待在途请求，再提交一次仍有必要的 `CancelFishing`；自然结束不补取消。`GameSession.whenIdle()` 同时覆盖尚未返回的钓鱼启动和付费取消命令。保存、加载、新建、菜单和退出等待这条边界；取消失败不保存或离开，停止入口仍可重试。销毁停止续发；正常退出在销毁前完成取消。

原生加载在应用快照前释放存储 busy 锁，合法钓鱼存档可恢复；界面语言同步完成后再开始下一步。会话重置清理目标、旅行目的地、骑乘和挖掘待方向；旧计时器／挖掘响应不能重新提交或挂起输入。挖掘命令在途时也可按 Escape 阻止失败后的再次选向，已提交动作仍保留。

K1 的 F1 修正为 WASD 小写 `c` 向东南、`Shift+C` 关门，中英文提示同步；F2／F3／F6／F7 的修饰键、IME、可编辑上下文和重复输入在地图、玩家页、物品列表及装置菜单按各自上下文处理。连续行动先消费取消；方向／目标模式只消费当前层；空闲 Escape 不发命令。F8 的钓鱼停止按键不再穿透到手动动作。F4／F5 的其他大小写键义和 vi 的 B／J 优先级保留既有约定，完整原版预设和跑步仍是后续项。F9 已由 K2／K3 处理。能力待方向、激活物品和体内装置的显式取消仍提交原有核心命令；载入清理不伪造一次旧物品使用。

验证命令（前端在 `web`）：

```powershell
node --test src/input-controller.test.ts src/game-session.test.ts src/save-panel.test.ts src/player-ui-layout.test.ts src/object-list.test.ts src/magic-eater-panel.test.ts src/duelist-panel.test.ts src/status-panel.test.ts src/terrain-interaction.test.ts src/targeting.test.ts src/localization.test.ts
npm run typecheck
cargo test -p rfb-core --lib -- game::tests::asgard::asgard_fishing_start_block_save_cancel_and_ecology_resume game::tests::magic_eater::usage::empty_sp_is_checked_after_failure_and_cancellation_refunds_time_without_rng_rollback
```

首轮前端 118 项通过；补入载入时语言同步及原生存储解锁边界后，重新运行输入／保存 55 项和类型检查通过，当前覆盖合计 119 项、无跳过。核心既有钓鱼零时间取消／保存恢复、体内装置取消费用与 RNG 两项通过。K4 没有新增 Rust 规则、协议、内容版本或存档／哈希格式变动，未刷新 fixture；原版来源仍为上述 master 提交。桌面原生实按、窄屏／焦点和普通 standalone 交付留在 K5。

以下保留设计与完成条件：

- 取消键优先处理当前连续行动，再由当前对话框／方向／目标选择消费；空闲地图上的 Escape 不产生核心动作。不让一个 Escape 同时穿透多个交互层。
- 沿用钓鱼已存在的 `ContinueFishing`／`CancelFishing`：用户取消后在当前请求结束时提交一次必要的零时间取消，保存／退出不能抢在它之前。不能只停前端计时器而把仍在钓鱼的状态保存下来。
- 保留原有合法钓鱼存档的恢复行为；“旧会话残留定时器”和“新载入存档确实处于钓鱼状态”分开处理。被用户明确取消后保存的状态不得再次自动钓鱼。
- 清理挖掘失败后的待方向、旅行选点、骑乘选向等本地状态。能力／装置瞄准已有的付费取消规则仍由各自消费者执行，不能为统一 Escape 一律改成免费取消。
- 根据 K1 修正修饰键误触及本批入口冲突；不顺手把当前 vi 预设改成原版 Roguelike，或重映射所有现有按钮。
- 中英文提供当前连续行动、停止操作和停止原因；自动循环期间停止入口始终可达。底层逐步事件保留，界面避免每个休息回合重复刷“已完成”通知。

**完成条件：**地图、对话框、文本框、方向／目标选择和 busy 状态没有取消穿透；切换会话后旧循环不再提交命令；钓鱼和有费用的瞄准取消不回归。

## K5：定向验证与交付

优先扩充已有 [输入测试](../web/src/input-controller.test.ts)、[派发测试](../web/src/game-session.test.ts) 和实际面板测试。用可控制完成时机的 Promise 覆盖在途取消、两步间取消、错误和旧回调；复用现有时间／DOM 测试方式，不另建测试框架。

| 验证层 | 必须证明的行为 |
| --- | --- |
| 输入与调度 | 三种当前预设的正常操作保持；修饰键、IME、文本框与对话框不误发命令；最多一请求在途；取消输入不连带执行新动作 |
| 旅行／拾取 | 取消后不续发；`J` 合法恢复；自动取物外层换目标也不能越过取消；阻路、敌人、受伤、错误和待选择都停 |
| 休息核心 | K3 列出的恢复／时钟／停止条件及真实调用者；相同单步流的保存恢复和回放一致性 |
| 会话与钓鱼 | 停止后保存、加载、新建、返回菜单、失焦、销毁；旧响应不污染新会话；钓鱼取消提交恰好一次 |
| 桌面 | 普通新建角色，准备最小安全路径、可拾物和恢复需求；局部旅行、世界旅行、拾取、休息及钓鱼通过原生按键／停止按钮取消，随后执行手动动作并原生保存恢复 |

前端在 `web` 使用现有命令；只运行实际改动对应的测试文件：

```powershell
node --test src/input-controller.test.ts src/game-session.test.ts
npm run typecheck
```

Rust 聚焦休息及受影响消费者，复用 `game::tests::abilities::restoration`、`game::tests::hunger`、`game::tests::pet_upkeep` 和实际涉及的召回／体内装置／待选择用例；选完整测试名或明确模块路径，不用 `rest` 过滤整个仓库。只有改变对应类型时才生成协议／Schema；不为纯输入变动升级内容包或刷新内容锁。

如果 K3 改变公共命令结算、状态哈希输入、RNG 或共享投影，依照[验证与契约](testing.md)执行相应全局契约验证，解释每处差异后才刷新。相同命令序列无变化时，保留现有 fixture。相关检查通过后不重复扩大为完整 workspace 或 Android 验收。

本批交付要求一次聚焦 Windows 桌面验收，因为“按下停止是否真的停”不能只由纯函数测试证明。复用现有 Tauri E2E 助手和最小状态准备；执行 `npm run e2e:build` 后运行实施时新增的专项入口，并用 `npm run build:standalone:debug` 交付普通可玩产物。新增入口的准确命令在实现后登记，不把尚不存在的参数写成现有命令。

交付清单：

1. 完整命令／键位对照，列明本批修正和仍待实现的命令。
2. 五类已有连续行动的取消及会话边界实现；待方向挖掘等不再残留。
3. 自动测试与桌面证据，注明准备条件、取消时实际完成的动作和保存恢复结果。
4. 相关版本／契约影响说明、普通 Tauri standalone 产物位置及剩余限制。

K1–K5 按依赖顺序推进，每批记录事实后更新本计划状态。完成前不得将“已规划”“已有停止判断”或“构建成功”写成完整键位兼容／取消验收通过。
