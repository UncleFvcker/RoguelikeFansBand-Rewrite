# K1：原版命令、默认键位与当前入口对照

核对日期：2026-09-13。下表保留 K1 基线的静态审计事实；K1 本身没有修改游戏代码、运行游戏测试或进行逐键桌面验收。后续 K2–K4 已实现旅行／取物／休息／钓鱼取消及输入上下文修复并通过定向自动验证，K5 桌面验收待实施；F1–F9 的修复与保留项见[实施计划](command-input-cancellation-plan.md#k4输入上下文钓鱼和生命周期收口)。表内“当前”及行号均指下述 K1 基线，不能用其已修复缺口描述后续代码。

已接功能的 Original／Roguelike 快捷入口已在后续批次实施，最新可用键见[当前键盘操作](keyboard-controls.md)；下表保留 K1 时的缺口记录。戒指交换 Original `W`／Roguelike `\W` 已接原子交换及多栏位选择，见[操作说明](keyboard-controls.md#戒指交换)。特殊行走 `-`、普通行走选向 `;` 与 Original `,`／Roguelike `.` 停留已在后续批次接入；默认拾取设置、翻转语义及保留的原版差异见[当前键盘操作](keyboard-controls.md#特殊行走与停留)。

后续宠物菜单 `p` 已接指定目标、按只解散、源距离指令与骑乘入口，范围及尚未完成的高级开关见[宠物菜单说明](keyboard-controls.md#宠物菜单)。包裹直接增加栏位，按用户决定不做独立菜单和 `P` 键；该项属于设计排除，下表历史“入口缺失”不再代表待办。

## 基线与读表规则

- 当前实现：`main@73e760297da43d4ef9c9546a697b4e51ce6c3e00`；原有计划文档及未跟踪的 `release/` 保留。
- 原版：`D:/codex/Frogcomposband/master` 的 `master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，全部通过 `git show`／`git grep` 读取 Git 对象。
- 本表覆盖 `_dispatch_command`、`process_command` 的所有 case／default、输入层的计数／重复／录制／转义，以及两套默认 `C:` 映射。各职业、商店和知识菜单内部的具体内容不逐项重做，但其通用入口和输入上下文有去向。
- `D:行号` 指该来源提交的 `src/dungeon.c`；`K:行号` 指 `lib/pref/pref-key.prf`；`U:行号` 指 `src/util.c`。例如 `git -C D:/codex/Frogcomposband/master show a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:src/dungeon.c` 可复核分发。不能用原版工作树当前文件的行号替代。
- Original／Roguelike 列记录**未加个人覆盖时的有效字符输入**，包含默认映射展开。`\b` 表示先输入反斜杠绕过映射，再输入 `b`；不是要求同时按两个键。方向记为 `dir`，范围由各命令检查。
- 当前键位按地图空闲、无其他对话框、允许游戏操作的普通状态说明；“无键”表示未接该命令快捷键，不表示相同字符一定没有其他动作。例外在输入上下文和冲突表中列出。
- 分类：**入口已有**＝当前范围的规则／投影及 UI 可达，不代表原版全部语义完成；**缺键**＝已有功能缺对应快捷入口；**键义不同**＝同键指向另一功能；**规则缺失**＝没有该通用命令消费者；**平台适配**＝当前 UI 用另一种方式承担；**开发专用**＝不应当直接变成普通玩家动作。
- 后续归属：K2 旅行／取物取消，K3 休息取消，K4 输入上下文及冲突；“后续通用”指本批之后的键位／通用系统；“内容方向”只涉及缺失身份或具体内容消费者，不能用于推迟本批公共取消。

## 先核对映射，再解释命令

原版加载顺序为 `init2.c:1692–1701` 的 `pref.prf` 与 `pref-$SYS.prf`，随后 `dungeon.c:5609–5703` 加载 user／平台、种族、职业、角色和领域配置，最后加载 `keybind.prf`。`pref.prf` 包含 `pref-key.prf` 和 `pref-opt.prf`；后者默认关闭 `rogue_like_commands` 和 `command_menu`，开启 `always_repeat`。因此表中的 Roguelike 是切换到该预设之后的默认映射，不是普通新游戏默认已启用。

`request_command` 只展开一次 keymap（U:4110 起），不会把展开产生的动作再次当作用户按键映射。个人配置可能覆盖本表；本次只核对仓库默认配置，不推测本机原版用户存档或未跟踪的个人配置。

有三处需要明确：

1. `D:4074` 的 `Z` 实际调用自动探索，不是帮助文件所写的铭刻，也不是旧 Roguelike 帮助所写的使用 staff。
2. 此提交的 Roguelike 默认没有把 `b` 浏览和 `u` 使用 staff 搬到别的普通键；二者被移动映射占用，可通过 `\b`／`\u` 调用。`P` 的实际分发是包裹菜单，不能按旧帮助写成浏览法术。
3. `cmd4.c:1640–1705` 的游戏内键位编辑器另有“默认键”元数据，部分条目与 `pref-key.prf` 冲突，如 Roguelike 的 `b/u`。它由选项菜单进入，重置动作会修改 keymap；不能把编辑器表当作启动时的有效键位。

### 两套默认 C 映射的完整清单

下列每个来源行均对应一条实际 `C:`，分组只压缩相同动作族。未出现的字符直接进入命令层；控制字符另受底层输入处理。

| 预设／按键 | 展开结果 | 来源 | 去向 |
| --- | --- | --- | --- |
| Original `5` | `,` | K:16 | 停留／拾取 |
| Original `1 2 3 4 6 7 8 9` | `;1 ;2 ;3 ;4 ;6 ;7 ;8 ;9` | K:20, K:22, K:24, K:26, K:28, K:30, K:32, K:34 | 八方向行走 |
| Original Ctrl+J | 回车 | K:38 | 空命令；开启命令菜单时另见输入前处理 |
| Original Ctrl+K、Ctrl+C | `Q` | K:42, K:46 | 自杀命令，仍经过源确认 |
| Roguelike `X` | `n` | K:52 | 重复上次命令 |
| Roguelike `.`、`,`、`5` | 分别 `,`、`.`、`,` | K:56, K:60, K:64 | 停留、跑步、停留；以动作值为准，不依赖文件注释 |
| Roguelike `1 2 3 4 6 7 8 9` | 同 Original 数字方向 | K:68, K:70, K:72, K:74, K:76, K:78, K:80, K:82 | 八方向行走 |
| Roguelike `b j n h l y k u` | `;1 ;2 ;3 ;4 ;6 ;7 ;8 ;9` | K:86, K:88, K:90, K:92, K:94, K:96, K:98, K:100 | 八方向行走 |
| Roguelike `B J N H L Y K U` | `.1 .2 .3 .4 .6 .7 .8 .9` | K:104, K:106, K:108, K:110, K:112, K:114, K:116, K:118 | 八方向跑步 |
| Roguelike Ctrl+`B J N H L Y K U` | `+1 +2 +3 +4 +6 +7 +8 +9` | K:122, K:124, K:126, K:128, K:130, K:132, K:134, K:136 | 相邻综合交互，不能一律称为挖掘 |
| Roguelike Ctrl+T | `T` | K:140 | 挖掘 |
| Roguelike Ctrl+D | `k` | K:144 | 摧毁 |
| Roguelike `W` | `L` | K:148 | 地图定位 |
| Roguelike `S`、`#` | 分别 `j`、`S` | K:152, K:156 | 钉门、持续搜索 |
| Roguelike `T`、`t`、`f` | 分别 `t`、`f`、`B` | K:160, K:164, K:168 | 脱装、射击、撞门 |
| Roguelike `x` | `l` | K:172 | 查看 |
| Roguelike `z`、`a` | 分别 `a`、`z` | K:176, K:180 | 使用 wand、rod |
| Roguelike Ctrl+C | `Q` | K:184 | 自杀命令，仍经过源确认 |
| Roguelike `O` | `U` | K:188 | 种族／职业能力 |
| Roguelike `(` | `J` | K:192 | 继续旅行 |
| Roguelike Ctrl+E | `H` | K:196 | 旅行到最近的合适未知物品 |

### 平台宏的范围

平台文件通过 `P:` 把扫描码或终端序列变成上述字符／动作；本次检查的 `pref-*` 平台文件没有额外 `C:` 覆盖。`user.prf` 含条件平台加载，仓库中的 `user-win.prf` 只重定义颜色。源仓库没有本次可用于假定个人覆盖的 `keybind.prf`。

| 平台文件 | 实际动作族及差异 | 重写版去向 |
| --- | --- | --- |
| `pref-win.prf` | 普通小键盘／导航扫描码到数字、`/ * - + 0 .` 等；Shift+方向宏先 Escape 再绕过映射执行 `.dir`，Shift+5 回中；Ctrl+方向执行 `+dir`；含 US／Japan／NEC98 扫描码条件和旧 DOS 变体 | 现有 `KeyboardEvent.code` 仅接小键盘八方向和 5；Shift／Ctrl 跑步与综合交互未接。扫描码协议本身不移植到 WebView |
| `pref-x11.prf`、`pref-sdl.prf` | 数字方向、绕过映射的 `.dir`／`+dir`、Ctrl+V；有以 0 作为后续方向输入的动作 | 动作归到同一通用清单；不是新增另一套规则 |
| `pref-gcu.prf` | 终端方向／导航序列映射到 `.dir`，并有 `.`、`0`、`5`、Escape | 与 Windows 原始序列不同；不据此把所有平台箭头都宣称为单步移动 |
| `pref-mac.prf` | 数字／标点、回车、Escape；跑步／综合交互及带 `w0` 的旧换装组合宏 | 单步动作按清单；旧组合宏列后续宏功能，不移植旧终端编码 |
| `pref-emx.prf` | 包含 `pref-win.prf`，另有 `R`＋回车宏 | 休息消费者同表；无本次平台实机验收 |
| `pref-acn.prf` | 四方向数字和 Ctrl+S | 对应移动／保存；无本次平台实机验收 |
| `pref-ami.prf` | 无动作映射 | 无新增命令 |

这里完成的是仓库平台宏到命令族的静态归属，不宣称所有历史硬件键、NumLock 或操作系统保留组合都已实测。Windows 是 K5 的本批原生输入验收平台。

## 当前消费者与既有证据索引

表中的 C 编号指下列真实入口／消费者；T 编号只表示查到了既有测试，不代表本次重跑或覆盖全部原版语义。无此行为的项目不借用相邻测试充当完成证据。

| 编号 | 当前代码／测试依据 |
| --- | --- |
| C1 | [地图键盘分发](../web/src/input-controller.ts)、[地形键位](../web/src/terrain-interaction.ts)、[命令协议](../crates/rfb-protocol/src/lib.rs)、[核心分发](../crates/rfb-core/src/game/mod.rs) |
| C2 | [局部旅行](../crates/rfb-core/src/game/travel.rs)、[世界旅行](../crates/rfb-core/src/game/wilderness.rs)、[自动取物](../crates/rfb-core/src/game/mogaminator.rs) |
| C3 | [恢复休息](../crates/rfb-core/src/game/player_abilities.rs)、[状态面板休息入口](../web/src/status-panel.ts)、[预算常量](../web/src/rest.ts) |
| C4 | [地形规则](../crates/rfb-core/src/game/terrain.rs)、[宝箱规则](../crates/rfb-core/src/game/chests.rs)、[物品列表和宝箱按钮](../web/src/object-list.ts) |
| C5 | [背包／装备面板](../web/src/inventory-panel.ts)、[装备／铭刻／摧毁规则](../crates/rfb-core/src/game/inventory.rs)、[物品使用](../crates/rfb-core/src/game/item_use.rs) |
| C6 | [角色页面和 i/m 快捷键](../web/src/player-ui-layout.ts)、[能力／学习按钮及角色信息](../web/src/status-panel.ts)、[领域学习](../crates/rfb-core/src/game/spell_realms.rs) |
| C7 | [食魔者装置菜单](../web/src/magic-eater-panel.ts)、[核心食魔者](../crates/rfb-core/src/game/magic_eater.rs)；`#projection` 是 `player.magicEater`，不能推广成所有职业共享装置快捷键 |
| C8 | [目标光标](../web/src/targeting.ts)、[玩家射击／投掷](../crates/rfb-core/src/game/player_combat.rs)、[怪物探查面板](../web/src/monster-probe-panel.ts) |
| C9 | [宠物面板](../web/src/status-panel.ts)、[维持／解散](../crates/rfb-core/src/game/pet_upkeep.rs)、[骑乘规则](../crates/rfb-core/src/game/riding_proficiency.rs)、[命令分发](../crates/rfb-core/src/game/mod.rs) |
| C10 | [显示设置](../web/src/settings-panel.ts)、[旅行设置与界面组装](../web/src/main.ts)、[Mogaminator 编辑器](../web/src/mogaminator-editor.ts) |
| C11 | [保存界面](../web/src/save-panel.ts)、[原生存储](../web/src/native-save-storage.ts)、[会话菜单](../web/src/session-shell.ts)、[导出保存／诊断回放](../web/src/main.ts)、[Tauri](../web/src-tauri/src/lib.rs) |
| C12 | [任务／胜利／退休](../crates/rfb-core/src/game/tasks.rs)、[商店](../web/src/shop-panel.ts)、[Home／博物馆](../web/src/home-panel.ts)、[设施](../web/src/task-service-panel.ts) |
| C13 | [消息面板](../web/src/message-panel.ts)、[角色信息](../web/src/status-panel.ts)、[相机](../web/src/camera.ts)、[渲染](../web/src/map-renderer.ts) |
| T1 | [input-controller.test.ts](../web/src/input-controller.test.ts)：预设映射、Ctrl+G、Shift+O、已知楼梯、旅行受伤／敌人／阻路、坐标平移、锁定拾取目标、钓鱼取消和强制方向。预设用例主要直接调用映射函数，不能证明完整事件分发无冲突 |
| T2 | [terrain-interaction.test.ts](../web/src/terrain-interaction.test.ts)：大小写地形键和失败后待方向；[targeting.test.ts](../web/src/targeting.test.ts)：当前光标／目标选择 |
| T3 | [player-ui-layout.test.ts](../web/src/player-ui-layout.test.ts)：i/m、编辑焦点、其他对话框、页面 tab 和背包子对话框 |
| T4 | [inventory-panel.test.ts](../web/src/inventory-panel.test.ts)、[object-list.test.ts](../web/src/object-list.test.ts)、[magic-eater-panel.test.ts](../web/src/magic-eater-panel.test.ts)：各面板现有交互；不是全职业原版物品标签测试 |
| T5 | [world.rs](../crates/rfb-core/src/game/tests/world.rs)：世界地图时间、往返和伏击；[mogaminator.rs 内嵌测试](../crates/rfb-core/src/game/mogaminator.rs)：单步取物、过期 ID、隐藏金币；[restoration.rs](../crates/rfb-core/src/game/tests/abilities/restoration.rs)：恢复休息 |
| T6 | [game-session.test.ts](../web/src/game-session.test.ts)：busy、失败、终态、世界地图及待选择阻断；[Tauri 测试](../web/src-tauri/src/lib.rs)和[回放测试](../crates/rfb-replay/src/tests.rs)：既有保存／回放，不是玩家录制宏 |

## 命令逐项对照

### 移动、旅行、搜索与地形

| 命令 | Original／Roguelike 有效键 | 源分支／消费者 | 当前规则、入口与键 | 分类／后续；证据 |
| --- | --- | --- | --- | --- |
| 行走 | `;`＋dir／同；数字八方向／数字或 `hjklyubn` | D:3703 `do_cmd_walk`；`cmd2.c:2283` | C1 `Move`；numpad code／vi／WASD；移动撞怪进入现有近战 | 入口已有，预设并非原版；WASD c 冲突交 K4；T1 |
| 翻转拾取的行走 | `-`＋dir／同 | D:3718 `do_cmd_walk` 的不同 pickup 参数 | `Move` 无本次动作的翻转拾取参数；没有 `-` 入口 | 规则缺失／后续通用；无该语义测试 |
| 停留／按设置拾取 | `,`、`5`／`.`、`5` | D:3745 `do_cmd_stay`；`cmd2.c:2412` | C1 `Wait`；numpad 5、vi `.`、WASD Space；`PickUp` 独立 | 部分适配，不能把 Wait 算完整 stay/pickup；T1 |
| 跑步 | `.`＋dir／`,`＋dir 或大写方向 | D:3737 `do_cmd_run`；`cmd2.c:2369` | 无 Run；大写方向并非跑步 | 规则缺失／后续通用；无 |
| 拾取脚下物品 | `g`／同 | D:3753 `do_cmd_get` | C1 `PickUp`，`g/G` | 入口已有但 `G` 与学习冲突；后续键位；T1 |
| 自动取物 | Ctrl+G／同 | D:3761 `do_cmd_autoget`；`cmd2.c:2580` | C1/C2 `autoGet()` 先 PickUp，再串行 AutoGet，核心选目标 | 入口已有，缺主动取消／K2；T1/T5 |
| 到最近合适未知物品 | `H`／Ctrl+E | D:3766 `do_cmd_get_nearest`；`cmd2.c:4147` | M3 已接 FindNearestUnknownItem／TravelUnknownItem，按当前实例鉴定／感觉与规则筛选，并以加权路线成本选定一件；键位及全预设按钮开放 | 核心／入口／定向验证已完成，桌面待 K5；当前知识模型与源排序适配见 M3 |
| 休息 | `R`／同 | D:3772 `do_cmd_rest`；`cmd2.c:2604` | 已接次数／*／& 选择，默认 &；固定回合、资源恢复、含异常及倒计时的完整恢复分别由 Core 处理 | 模式、逐步取消及上一命令复用已接；保留适配见键盘操作，桌面待 K5 |
| 单次搜索 | `s`／同 | D:3779 `do_cmd_search`；`cmd2.c:258` | C1/C4 `Search`，大写 `S`；小写 s 在 WASD 是南行 | 键义不同；K4 保持现行方向职责，原版键位后续；T2 |
| 持续搜索 | `S`／`#` | D:3786 `ACTION_SEARCH` 切换 | 无持续搜索状态／命令，当前 S 仅单次搜索 | 规则缺失／后续通用；无 |
| 相邻综合交互 | `+`＋dir／同或 Ctrl+方向 | D:3689 `do_cmd_alter`；`cmd2.c:2069` | 后续已接 Alter：按怪物、开门、撞门、挖掘、关门、拆除顺序选择；+ 选向及 Ctrl 方向 | 规则与快捷入口已接；范围、自动验证与保留适配见 [当前操作](keyboard-controls.md)，原生实按待 K5 |
| 挖掘 | `T`／Ctrl+T | D:3696 `do_cmd_tunnel`；`cmd2.c:1372` | C1/C4 `DigTerrain`，T／Rogue Ctrl+T 后选一次方向；可重试失败自动续发 | 已接默认 99 次、显式次数覆盖和逐步取消；桌面实按待 K5 |
| 开门／开箱 | `o`／同 | D:3836 `do_cmd_open`；`cmd2.c:935` | C4 `OpenDoor` 用 o；`OpenChest` 由物品列表按钮，未复用 o 的箱子选择 | 部分入口／后续键位；T2/T4 |
| 关门 | `c`／同 | D:3843 `do_cmd_close`；`cmd2.c:1110` | C4 `CloseDoor`，c/C；地图上优先于 WASD c 方向 | 入口已有，冲突交 K4；T2 |
| 钉门 | `j`／`S` | D:3850 `do_cmd_spike`；`cmd2.c:2172` | SpikeDoor 已接两套选向；铁蒺藜可在七镇杂货店购买，门逐级加固 | 已接规则与键位；桌面实按待 K5 |
| 撞门 | `B`／`f` | D:3857 `do_cmd_bash`；`cmd2.c:1982` | C4 `BashDoor`，大写 B 后方向 | Original 入口近似；vi B 不跑步，Rogue f 当前射击；T2 |
| 拆陷阱／箱子陷阱 | `D`／同 | D:3864 `do_cmd_disarm`；`cmd2.c:1762` | C4 `DisarmTrap` 用 D；`DisarmChest` 由物品列表按钮 | 部分入口／后续键位；T2/T4 |
| 上楼／地表大地图 | `<`／同 | D:3805 `do_cmd_go_up`／`change_wild_mode` | C1 `TraverseStairs` 或 `EnterWorldMap`，< 及按钮 | 入口已有，宠物／召回确认已有；上下楼同用 TraverseStairs，非两条定向命令；T1/T5 |
| 下楼／退出大地图 | `>`／同 | D:3825 `do_cmd_go_down`／`change_wild_mode` | C1 `TraverseStairs` 或 `LeaveWorldMap`，> 及按钮 | 入口已有，当前 <、> 在楼梯上均落同一 TraverseStairs；T1/T5 |
| 指定位置旅行 | 反引号／同 | D:4469 `do_cmd_travel`；`cmd2.c:4140` | C1/C2 `TravelLocal`：反引号选点／物品列表旅行；世界地图 x 查看后 Enter 为 TravelWorld | 入口已有，局部仅已探索位置；取消交 K2；T1/T5 |
| 继续旅行 | `J`／`(` | D:4475 `travel_begin` 前资格和确认 | C1 有本地目的地时 J 继续，无确认；世界地图 J 也可继续；地图 `(` 无入口 | 键义／确认适配；K2 保留合法目的地；T1 未覆盖用户取消 |
| 自动探索 | `Z`／同 | D:4074 `do_cmd_auto_explore`；`cmd2.c:2505–2571` | M2 已接 AutoExplore／ContinueAutoExplore／CancelAutoExplore，核心选择可达边缘并优先处理已启用取物；两套 RFB 预设 Z，全预设按钮，旧 WASD Z 保留 | 核心／入口／定向验证已完成，桌面待 K5；见 M2 计划 |

### 物品、法术菜单与宠物入口

| 命令 | Original／Roguelike 有效键 | 源分支／消费者 | 当前规则、入口与键 | 分类／后续；证据 |
| --- | --- | --- | --- | --- |
| 穿戴／挥舞 | `w`／同 | D:3615 `equip_wield_ui` | C5 `Equip`，背包选择装备／槽位；无 w 命令，WASD w 是北行 | 缺键／后续通用；T4 |
| 脱装 | `t`／`T` | D:3622 `equip_takeoff_ui` | C5 `Unequip`，装备按钮；无 t，当前 T 是挖掘 | 缺键／键义不同；T4 |
| 丢弃 | `d`／同 | D:3629 `do_cmd_drop` | C5 `Drop`／`DropQuantity`，数量确认；无 d 命令，WASD d 是东行 | 缺键／后续通用；T4 |
| 摧毁 | `k`／Ctrl+D | D:3636 `obj_destroy_ui` | C5 `DestroyItem`，更多操作；vi k 是北行 | 缺键；Ctrl 穿透交 K4；T4 |
| 装备列表 | `e`／同 | D:3643 `equip_ui` | C5 背包页内装备区域；无独立 e 快捷入口，WASD e 是东北行 | 缺键／平台适配；T4 |
| 背包列表 | `i`／同 | D:3650 `pack_ui` | C6 `i/I` 打开背包，无核心动作 | 入口已有；纠正初查可能遗漏的页面快捷键；T3 |
| 交换戒指手指 | `W`／`\W` | D:3655 `ring_finger_swap_ui` | C5 有按槽位装备；无原子交换两戒指命令／按钮 | 规则缺失，不能以两次摘戴冒充；后续通用 |
| 检视物品 | `I`／同 | D:3665 `obj_inspect_ui` | C5 点物品详情；I 打开背包而非检视选择 | 功能已有、键义不同／后续键位；T3/T4 |
| 切换背包／装备和列表窗口 | Tab（Ctrl+I）／同 | D:3672 `toggle_inven_equip`／`toggle_mon_obj_lists` | DOM Tab 焦点、页面标签切换；无原版窗口切换动作 | 平台适配；不抢占标准 Tab；T3 |
| 包裹菜单 | `P`／同 | D:3680 `bag_ui` | C5 有装备袋容量消费者，未见手动放入／取出包裹菜单或 GameCommand | 通用容器入口缺失；具体袋效果属内容方向；无此 UI 测试 |
| 学习 | `G`／同 | D:3874 `do_cmd_study`／身份分支 | C6 `StudyAbility`／`StudyPrayer` 等已有按钮，C7 吸收入口；G 当前拾取 | 缺键／键义不同；未开放身份不在本批补齐；T4 |
| 浏览法术／技能 | `b`／`\b` | D:3898 `do_cmd_browse`／`do_cmd_spell_browse` 等 | C6 能力详情与书本页，C7 体内装置；无通用 b，vi b 移动 | 功能已有但缺专门键；后续键位／内容方向 |
| 施法／技能 | `m`／同 | D:3941 `do_cmd_cast`／`do_cmd_spell` 等身份分支 | C6 m/M 打开能力页，按钮发 `CastAbility`；食魔者由 C7 m 优先打开装置菜单 | 入口已有、细节按已开放身份；非完整原版选书／标签操作；T3/T4 |
| 种族／职业能力 | `U`／`O` | D:4230 `do_cmd_power` | C6 统一能力页，`CastAbility`；无独立 U/O，Shift+O 当前物品列表 | 缺键／键义不同；后续键位，具体身份属内容方向 |
| 激活物品 | `A`／同 | D:4094 `do_cmd_activate` | C5 `UseItem` 等激活按钮和目标选择；无通用 A；WASD A 移动，食魔者还可能打开 wand | 缺键／上下文冲突；T4 |
| 进食 | `E`／同 | D:4110 `do_cmd_eat_food` | C5 `UseItem`，背包使用按钮；无 E，WASD E 东北行 | 缺键／后续键位；T4 |
| 光源补充燃料 | `F`／同 | D:4117 `do_cmd_refill` | C5 `RefuelLight`，装备行按钮；F 当前进入射击瞄准 | 功能已有、键义不同；T4 |
| 射击 | `f`／`t` | D:4124 `do_cmd_fire` | C1/C8 `FireTarget`，f/F 和目标按钮；方向 Fire 协议也已有 | 部分入口；原版 Rogue t 未接；T1/T2 |
| 投掷 | `v`／同 | D:4131 `py_throw` | C5/C8 背包更多操作进入投掷选向，`Throw`；v/V 当前骑乘 | 功能已有、键义不同；T4 |
| 使用 wand | `a`／`z` | D:4142 `do_cmd_aim_wand` | C5 使用按钮；仅食魔者 C7 的 a/A（必要时 Alt）打开 wand 菜单 | 部分键，不是所有角色都有 a；T4 |
| 使用 rod | `z`／`a` | D:4160 `do_cmd_zap_rod` | C5 使用按钮；仅食魔者 C7 的 z/Z（必要时 Alt）打开 rod 菜单 | 部分键；与源自动探索 Z／WASD 方向有冲突；T4 |
| 饮用药水 | `q`／同 | D:4178 `do_cmd_quaff_potion` | C5 `UseItem`；无 q 使用入口，WASD q 西北行 | 缺键／后续键位；T4 |
| 阅读卷轴 | `r`／同 | D:4196 `do_cmd_read_scroll` | C5 使用／阅读按钮，`UseItem` 等；r 当前休息 | 功能已有、键义不同；T4 |
| 使用 staff | `u`／`\u` | D:4214 `do_cmd_use_staff` | C5 使用按钮；仅食魔者 C7 的 u/U（vi 需 Alt）进入 staff 菜单 | 部分键；vi u 是东北行；T4 |
| 铭刻 | `{`／同 | D:4080 `obj_inscribe_ui` | C5 `InscribeItem`，更多操作；C7 体内槽局部 Z；无全局 `{` | 缺键／后续键位；T4 |
| 移除铭刻 | `}`／同 | D:4087 `obj_uninscribe_ui` | C5/C7 编辑铭刻置空／null，复用 `InscribeItem`；无 `}` | 功能已有但缺键；T4 |
| 指挥宠物 | `p`／同 | D:4068 `do_cmd_pet`；`cmd5.c:2509` | C9 四种 `SetSummonCommand`、全体 `DismissPets`；无 p；v/V 选向发 Ride | 部分实现；高级指挥后续，不能将骑乘键当投掷；已有状态投影 |

### 观察、知识、设置、保存与特殊命令

| 命令 | Original／Roguelike 有效键 | 源分支／消费者 | 当前规则、入口与键 | 分类／后续；证据 |
| --- | --- | --- | --- | --- |
| 完整局部地图 | `M`／同 | D:4247 `do_cmd_view_map` | 已接只读全图概览，支持当前局部／世界地图并保留迷雾 | 平台适配；见地图与情报操作说明 |
| 强制视口回中 | Ctrl+V／同 | D:4253 `viewport_verify_aux` | 已接一次性回中，覆盖跟随相机、全图滚动及定位窗口 | 已接，不进入骑乘 |
| 地图定位／滚动 | `L`／`W` | D:4261 `do_cmd_locate` | 已接独立定位窗口、半页方向平移、回中与取消 | 平台适配；不移动玩家 |
| 查看 | `l`／`x` | D:4268 `do_cmd_look` | C1/C8 x/X 和查看按钮；光标移动、说明；世界地图 Enter 启动旅行 | 入口已有但 Original 键不同；T1/T2 |
| 怪物列表 | `Y`、`[`／`[`（`\Y` 可直达别名） | D:4274, D:4275 `do_cmd_list_monsters` | 已接按距离排列的当前感知实例和位置，可转入定位窗口；幻觉时不列出 | 当前投影适配，不等于自动探查 |
| 物品列表 | Ctrl+O、`O`、`]`／Ctrl+O、`]` | D:4280, D:4281, D:4282 `do_cmd_list_objects` | C4 `]`／Shift+O 打开；Ctrl+O 被拒绝列表快捷键后可穿透成开门 | 部分键；误触交 K4；T1/T4 |
| 指定目标 | `*`／同 | D:4288 `do_cmd_target`，大地图时 look | C1/C8 f/F／目标按钮进入光标，确认 `FireTarget`；无全局 `*` | 部分入口；目标循环／旧目标后续；T1/T2 |
| 帮助 | `?`／同 | D:4300 `do_cmd_help` | 所有预设与按钮开放 12 主题中英文手册，按当前预设显示命令，支持搜索／目录／Esc | 当前实现操作帮助已接；不导入整套原版文档 |
| 查询字符 | `/`／同 | D:4307 `do_cmd_query_symbol` | 已接 94 项源图例、大小写字符、名称／唯一／非唯一／可骑乘过滤及持久曾见／调查资料 | 击杀排序在知识档案，逐次战斗 lore 抽样仍保留；见操作说明 |
| 角色信息 | `C`／同 | D:4314 `py_display` | C6 角色按钮与多页详情；C 当前关门 | 功能已有、键义不同／后续键位；T3 |
| 单行偏好命令 | `!`／同 | D:4325 `do_cmd_pref` | 无通用偏好解释器；C10 仅实际设置和 Mogaminator 源 | 后续配置；不伪造任意 prf 支持 |
| 重载自动拾取规则 | `$`／同 | D:4331 `do_cmd_reload_autopick` | C10 编辑器应用 `ConfigureMogaminator`；无源文件重载 $ | 平台适配／缺键；后续配置 |
| 编辑自动拾取规则 | `_`／同 | D:4337 `do_cmd_edit_autopick` | C10 `_` 和按钮，`ConfigureMogaminator` 等生产消费者 | 入口已有；编辑与查询对话框由自身处理；T4 |
| 宏编辑 | Ctrl+E、`@`／`@`（`\Ctrl+E` 可绕过旅行映射） | D:4344, D:4345 `do_cmd_macros` | 已接会话命令录制／步骤编辑／寄存器回放与自定义键位 | 非终端按键串／prf 宏，生命周期和支持范围见 keyboard-controls.md |
| 视觉编辑 | `%`／同 | D:4352 `do_cmd_visuals`＋redraw | C10 ASCII／图像 tileset 选择，无逐种类字形编辑 | 平台适配部分／后续配置 |
| 颜色编辑 | `&`／同 | D:4360 `do_cmd_colors`＋redraw | C13 使用现有配色，无用户调色入口 | 缺入口／后续配置 |
| 选项／键位设置 | `=`／同 | D:4368 `do_cmd_options`；`cmd4.c:2250` 可入键位编辑器 | 已接 = 与自定义键位按钮，五预设映射可保存／删除／清空／导入导出 | 映射到现有原版命令或会话寄存器，不递归 |
| 记笔记 | `:`／同 | D:4378 `do_cmd_note` | 已接本机笔记本、消息、附近已知地图、删除及文本导出 | 不随原生游戏存档迁移 |
| 版本信息 | `V`／同 | D:4385 `do_cmd_version` | 工程版本存在，无同义 V 界面；V 当前骑乘 | 缺入口／后续通用 |
| 楼层感知 | Ctrl+F／同 | D:4392 `do_cmd_feeling` | 已接 Core 当前层预感和任务／城镇／荒野说明；不穿透射击 | 即时查询适配，延时自动播报与幸运／宿敌变体保留 |
| 历史消息 | Ctrl+P／同 | D:4403 `do_cmd_messages` | C13 消息侧栏及清除按钮；无 Ctrl+P | 入口已有但缺键；完整历史查询后续 |
| 任务信息 | Ctrl+Q／同 | D:4410 `quests_display` | C6/C12 任务页按钮与任务状态；无 Ctrl+Q | 入口已有但缺键；WASD 组合误触交 K4 |
| 重绘 | Ctrl+R／同 | D:4417 `do_cmd_redraw` | C13 响应式渲染；无手动重绘命令；Ctrl+R 可变休息 | 平台适配；误触交 K4 |
| 保存不退出 | Ctrl+S／同（未定义 VERIFY_SAVEFILE 时） | D:4427 `do_cmd_save_game` | C11 原生保存／导出按钮；无 Ctrl+S，WASD 下可能南行 | 入口已有但缺键；误触／保存边界交 K4/T6 |
| 游戏时间 | Ctrl+T／`\Ctrl+T` | D:4435 `do_cmd_time` | C13 回合／world tick 等投影已有，无源日历显示快捷入口 | 部分信息／后续通用 |
| 保存并退出 | Ctrl+X／同；平台特殊退出事件 | D:4442, D:4443 `do_cmd_save_and_exit` | C11 保存、菜单退出流程已有，未接 Ctrl+X 的同义原子入口；组合可变查看 | 缺键／不能把分别保存和退出当成验收；K4/T6 |
| 自杀／结束角色 | `Q`、Ctrl+C、Ctrl+K／`Q`、Ctrl+C | D:4449 `do_cmd_suicide` | 无 Suicide 命令；C12 Retire 只在胜利及允许地点开放 | 规则缺失／后续产品决定；不把退休当自杀，不无确认接原版别名 |
| 知识菜单 | `~`／同 | D:4456 `do_cmd_knowledge`；`cmd4.c:8447` | 五组 22 个入口；已接角色持久物品／神器／Ego 档案、怪物见闻、存活与归属击杀、已到访地下城最深层数 | 搜索／详情／存活筛选已接；逐次战斗 lore 抽样与跨角色高分榜保留，见 keyboard-controls.md |
| 屏幕导出 | `)`／同 | D:4463 `do_cmd_save_screen` | 已接玩家地图 PNG／TXT／离线 HTML 导出与工具栏按钮 | 当前地图视图，不截取整个系统屏幕 |
| 空输入／取消 | Space、CR、LF、Escape；Rogue Ctrl+J 被方向映射覆盖 | D:3556, D:3557, D:3558, D:3559 | 空闲 Escape 无动作；WASD Space 是 Wait；对话框／选向另见下表 | 上下文适配；连续取消交 K2–K4 |
| 巫师开关 | Ctrl+Y、Ctrl+W／Ctrl+W（Ctrl+Y 已是 +7；可绕过） | D:3565, D:3566 `enter_wizard_mode` | 无普通入口；生产拒绝测试准备 IPC，不等于移植了巫师模式 | 开发专用，不列普通可玩缺口；T6 |
| 调试命令 | Ctrl+A／同；ALLOW_WIZARD 条件 | D:3591 `do_cmd_debug` | 无原版调试菜单；已有受限开发工具 | 开发专用；WASD Ctrl+A 误移动仍交 K4 |
| Spoiler 命令 | Ctrl+Z／同；ALLOW_SPOILERS／allow_spoilers 条件 | D:3605 `do_cmd_spoilers` | 无原版 spoiler 菜单；不是帮助所写的 Borg 开关 | 开发专用；组合误触交 K4 |
| 未识别命令 | 无映射且没有分支的字符 | D:4483 default，提示未知命令 | C1 大多忽略；方向／交互模式会提示不可用 | 平台适配；不随机输出原版错误趣味文本 |
| 进入任务场景 | 无普通单字符；特殊码 255 | D:3798 `do_cmd_quest`；`defines.h:308` | C12 已有任务入口／楼梯／设施链 | 自动入口；内容方向维护具体任务，不新增虚构键 |
| 进入商店／Home／博物馆 | 站上相应格产生特殊码 253 | D:4524 `shop_ui`／`home_ui`／`museum_ui`；`cmd1.c:4722` | C12 核心投影服务和前端面板、买卖存取命令 | 平台适配；已存在具体链，非全内容验收 |
| 进入设施 | 建筑流程特殊码 254 | D:4545 `do_cmd_bldg`；D:4950 产生建筑命令 | C12 设施面板和业务命令 | 平台适配；内容方向维护具体设施 |
| 常规分发包装 | 非 store／building 命令 | D:4548 default，`pack_lock` → `_dispatch_command` → `pack_unlock` | C1/C11 GameSession → 核心事务派发 | 基础设施对应，不是玩家快捷键；T6 |

### 命令分发之前的输入功能

| 功能／有效输入 | 原版实际来源与行为 | 当前状态／后续 |
| --- | --- | --- |
| `0`＋次数＋命令 | U:3986 起，0–9999 输入、空计数默认 99，支持退格；两预设相同 | 已接编辑／取消和串行次数执行；Core 投影是否可重复，跑步保存步数，休息支持固定回合，丢弃／摧毁使用数量；桌面实按待 K5 |
| 自动重复明显命令 | U:4137 起，`always_repeat` 给 `TBDoc+` 99 次预算；D:5024 消费 command_rep | 已接默认 99 次与显式次数覆盖，方向／箱子实例保持；Core 判定完成及中断，挖掘不再待方向；桌面实按待 K5 |
| `n`／Rogue `X` | U:4439 `repeat_check` 中上次命令寄存器 | 已接会话内上一游戏命令及选定参数；连续操作重启既有流程；vi／Rogue n 保留移动，详见键盘操作；桌面实按待 K5 |
| 单引号＋寄存器；双单引号列寄存器 | 同函数中 playback；ALLOW_REPEAT 条件 | 已接 62 个会话寄存器与点号上一命令；先显示列表，二次单引号保持列表 |
| 双引号＋寄存器＋命令 | 同函数中 record，后续选择参数进录制；Escape 可退 | 已接下一可重复命令与参数；Esc 取消空录制保留旧值 |
| 反斜杠＋命令 | U:4088 起，绕过 keymap，不再递归展开 | 五种预设均绕过自定义映射并按 Original 解释下一键 |
| `^`＋字符 | U:4100 起，显式生成控制字符，随后适用映射 | 无；WebView 可直接接真实 Ctrl，若要兼容前缀另立依据 |
| 命令菜单 | U:3976，开启 command_menu 且按键未映射时 CR/LF/x/X 调 `inkey_from_menu` | 本版固定 Enter／按钮入口，可搜索并执行原版操作，不抢占 x/X |
| 任意输入打断连续行为 | D:4700 起 `check_abort` 扫描 running/travel/repeat/rest/fish 等，flush 后 disturb | 现有旅行／取物／休息没有同等用户取消；钓鱼单独实现；K2–K4 |
| Ctrl+] | U:2487 起，底层 inkey 导出 doc/txt/html 屏幕，未进入 D 分发 | 已接同 ) 的 PNG／TXT／HTML 地图导出 |
| Ctrl+^、Ctrl+_ | U:2520 起，30 被剥离、31 作为宏触发标记；不是普通命令 | 终端编码不移植；真实 WebView 修饰键不能穿透成动作 |
| 购物模式输入转换 | U:4148 起，shopping==1 把 p/m 转 g、s 转 d | 当前 C12 用业务按钮／表单；没有通用字母采购菜单 |
| 装备铭刻命令确认 | U:4166 起，扫描装备铭刻的 `^` 及命令字符，确认后才派发 | 当前有部分物品销毁铭刻保护及各操作确认，不存在完整全局命令铭刻校验；后续通用／内容消费者核对 |

## 交互上下文与当前冲突

### 当前有效输入层次

`main.ts:491–504` 先安装 InputController，再安装 PlayerUiLayout。地图处理器先检查 busy／任意 open dialog／input、textarea、select，再处理目标、强制选择、地形／骑乘方向，随后才是地图快捷键。页面处理器还检查 defaultPrevented、repeat、修饰键、隐藏界面和 contentEditable，因此 i/m 不能只从 InputController 文件判断。

| 上下文 | 当前已接行为 | 尚缺／取消边界 |
| --- | --- | --- |
| 地图空闲 | numpad 八方向／5；vi 小写方向／`.`；WASD 方向／Space；g、r、地形键、x、f、v、旅行、列表、i/m | 并非 Original／Roguelike；无全局计数、宏、旧目标或连续取消 |
| 世界地图 | 当前方向预设、> 回局部；x 光标＋Enter 开始世界旅行；J 继续 | 不能把局部反引号入口照搬宣称已支持世界地图；运行期取消缺失 |
| 普通方向／骑乘选择 | 当前预设方向，Escape 撤销本地待输入 | busy 时 Escape 被上层直接挡住；修改应区分未提交选择和已提交动作 |
| 目标／查看／旅行选点 | 当前光标每次从玩家出发，方向移动；Enter 确认；旅行选点还支持 Space/`.`/5/0/t，< > 循环已记忆楼梯 | 源 `xtra2.c:4648` 的目标循环、自由／怪物模式、原地回中、旧目标、图鉴快捷等未全接；原版还接 q 取消和 t/`.`/5/0 确认，不能认为当前 Enter 已等价 |
| 强制方向选择 | 突变方向 Escape 被明确拒绝；能力后续方向 Escape 发 CancelAbilityDirection | 不是所有 Escape 都可免费撤销。`cancelTargeting` 对体内装置／物品激活另有结算；K4 必须保留 |
| 背包／物品使用 | i/I，DOM 焦点、按钮、详情、数量／铭刻／使用目标表单 | 没有通用原版 a–z 标签、`/` 切换背包／装备／箭袋及全套 @ 标签语义；C7 体内装置的标签支持不能推广到整个背包 |
| 物品列表 | ]／Shift+O；列表内 q/Escape 关闭，s 切楼梯，J、左括号、反引号发起旅行，导航键／名称首字母选择；宝箱按钮发生产命令 | 列表内修饰键过滤晚于 q/s/J 等分支，应纳入 K4；没有逐键原版列表等价声明 |
| 食魔者菜单 | m/a/u/z 在该角色投影存在时专门接入；局部标签、W/S/R 分类、X 换位、Z 铭刻 | 主地图方向优先：vi u、WASD a/z 要 Alt 才可让菜单取得输入；Escape 对待吸收有生产拒绝动作 |
| 页面和其他对话框 | i/m 切换当前玩家页；其他 open dialog 保有输入；DOM Escape 和标准 Tab/Enter/Space | 页面可在游戏 busy 时打开，不能把“打开对话框”当停止旅行；业务付费取消由对应面板负责 |
| 创角／保存／设施 | 现有表单、树形方向导航、确认／取消和业务按钮 | 属相应上下文，不给普通地图添加同名动作；本批不审计每个未开放职业或设施内部内容 |

### 已确认的输入差异／缺口

以下是源码可确认的派发结果或缺少的守卫；尚未做原生键盘复现。修饰组合仅在事件实际到达 WebView 时讨论，不保证所有 OS 组合都会送达。

| 编号 | 具体触发和代码依据 | 影响／归属 |
| --- | --- | --- |
| F1 | WASD 地图按 c：`terrainInteractionModeForKey` 在移动函数之前匹配 close-door，甚至没有可关门时也返回已处理 | 默认声明的东南移动键被截走；K4 需明确保留方向、另选关门入口，补完整事件测试 |
| F2 | 地图 Ctrl+R→rest、Ctrl+V→ride、Ctrl+F→射击、Ctrl+X→look；Ctrl+O 在列表过滤后又可匹配 open-door | InputController 没有统一 Ctrl/Alt/Meta 排除；K4 应修复修饰键穿透，不是全面重映射 |
| F3 | WASD Ctrl+S／Ctrl+A／Ctrl+Q／Ctrl+Z 等可进入方向分支；numpad code 也未排除修饰键 | OS／原版系统快捷键可能变成消耗回合动作；K4 |
| F4 | r/R 合并休息、g/G 合并拾取、f/F 合并射击、v/V 合并骑乘、c/C 合并关门；i/I 合并背包、m/M 合并能力页 | 明确标为键义不同；本批只修真实冲突，其余等完整原版预设，不能破坏当前 i/m 入口 |
| F5 | vi B 优先进入撞门；J 有旅行目标时继续，否则是南行；其他大写方向通常被 lower-case 当普通移动 | 现有 vi 仅移动预设，不能称完整 Roguelike；后续键位／跑步 |
| F6 | InputController 未检查 `isComposing`、`event.repeat`、`defaultPrevented` 或 contentEditable；PlayerUiLayout 只有后一组部分保护，也无 isComposing | 对真实可编辑上下文补一致规则；本次未发现需构造额外 contentEditable 控件，记录守卫差异即可；K4 |
| F7 | 物品列表 q/s/J/(/反引号早于修饰键排除；地图忙碌时先 return，页面 i/m 又独立处理 | 弹窗和 busy 的键行为不统一；K4；不能只修一个键盘处理函数 |
| F8 | 钓鱼 capture 监听仅置取消标志，不 preventDefault／stopPropagation | 同一次输入还可能进入普通动作；K4 需落实“停止一次、下一次才执行新动作”，保留核心 CancelFishing |
| F9 | 反引号选点和物品列表旅行共享 localTravelTo；世界旅行、取物各自循环；rest 是单次请求内多回合 | 单独给地图 Escape 添分支无法完成取消；K2/K3 按下表处理所有调用者 |

## 当前连续行动调用者清单

| 行为 | 全部生产启动／继续入口 | 核心边界与现有停止 | K2–K4 必须补的部分／既有证据 |
| --- | --- | --- | --- |
| 局部旅行 | InputController 的反引号选点确认、地图 J；ObjectList 的 J、左括号、反引号 → `main.ts:150` → `travelLocalTo`（824） | 每次 TravelLocal；位置／floor／mapScale 检查、受伤、可见敌人、混乱、决斗提示、阻路、抵达；坐标平移会更新目的地 | 无用户取消 token、无统一失焦／旧会话隔离；T1 只有自动停止／坐标和列表消费者证据 |
| 世界旅行 | 世界地图 x 查看后 Enter → `#travelWorldTo`（854）；世界地图 J | 每次 TravelWorld；抵达、地图尺度改变（含伏击）、无位移停止；目的地有核心投影 | 无主动取消和统一会话标识；T5 证明地图／伏击，不证明主动停止 |
| 自动取物 | 地图 Ctrl+G → `autoGet`（179）；无额外全局 AutoGet 按钮入口 | 首次 PickUp，外层选核心目标、内层逐条 AutoGet；敌人、受伤、盲／混乱、满包、查询、无进展等停止 | 取消必须同时终止内外两层，不能收完当前堆后转下一堆；T1/T5 |
| 恢复休息 | `commandForKeyboardInput`（1009）的 r/R；StatusPanel `#handleRest`（1146）的按钮 | 单条 Rest 9999；`player_abilities.rs:1938` 内循环，返回 RestResolution；核心处理资源／召回、敌人、伤害、死亡、待选择和预算 | K3 拆分并核对按命令／tick 结算；当前取消无法穿透一次请求；T5 不是拆分等价证据 |
| 钓鱼 | 现有物品／能力完成选向后核心出现 fishingDirection；每次 `reconcileStatus` → `#scheduleFishing`（737）→ `#advanceFishing`（746）；合法读档也会恢复调度 | 每次 ContinueFishing；keydown/pointerdown capture 标记，下一次发 CancelFishing；无方向则停止 | 保证当前请求后恰好一次取消、保存不抢先；区别合法读档和旧计时器；T1 有 busy 输入取消测试 |
| 地形／箱子自动重复 | 选向／箱子选择及物品列表 → `dispatchCounted`，默认 99 次 | Core 的 `commandRepeatable` 判定能否续发；完成、无效、危险或预算耗尽即停 | 已复用逐步取消、保存等待和会话隔离；不再重新等待挖掘方向；桌面实按待 K5 |
| 自动探测／地图与查询 | 核心 `prepare_local_travel` 内部消费；Mogaminator 查询由 main 中对话确认后派发 ResolveMogaminatorQuery | 属一条已提交生产命令／其要求的选择，不是独立前端旅行循环 | 当前命令必须完整结算，不能把取消实现成中断 RNG／物品扣费；K2/K4 保持选择边界 |

共用边界：`GameSession.dispatch` 返回 `Promise<void>` 并自行显示错误；循环无法直接区分拒绝与成功。`main.ts:771` 的 `showSessionView` 只显式取消 targeting／resetLocalTravel，不能据此宣称世界旅行、取物和休息已统一停止。`dispose()` 只清理既有监听／钓鱼计时器，尚无全部循环的会话失效协议。以上均是 K2–K4 的具体消费者，不需要新增持久任务系统。

## 本批结论、核验与后续

K1 已将原版分发、默认映射、平台宏动作族、输入前处理和现有连续行动调用者归入本表。没有把源帮助、源键位编辑器的默认元数据或当前方向映射函数单独当作最终玩家行为。

本次仅做文档核验：从源 Git 对象枚举的 96 个 case 标签／default 分支（包含别名，不代表 96 种命令）和 63 条默认 `C:` 映射，均在本文找到对应 D/K 来源定位；检查当前代码／测试文件引用、Markdown 链接、空白和工作树差异。没有执行游戏测试、构建、原版启动或逐键桌面验收；已有测试引用只是覆盖定位。

剩余明确边界：个人 prf 覆盖及历史平台硬件输入不作实机结论；复杂职业／物品／设施子菜单的全部内容不属 K1；来源自身的键位／帮助矛盾已记录，不替原版静默修正。

K1 给出的后续 K2／K3（旅行／取物取消与休息拆分）已完成实现与定向自动验证，证据登记在实施计划。接下来 K4 收口本表剩余输入冲突和钓鱼／页面／保存生命周期；K5 执行原生桌面实按验收。
