# 验证与契约

## 选择范围

检查的目标是证明本次行为正确并防止实际回归。先找已有覆盖，缺少证据时补测试；没有每个内容条目必须增加几个测试的配额。通过后交付，不因为仍有时间就扩大范围。

| 本次变化 | 选择的检查 |
| --- | --- |
| Markdown 文档 | 链接、命令、事实来源、`git diff --check`；不编译、不跑游戏测试 |
| 已有内容机制下的 JSON 参数/引用 | source/lock 验证及直接相关的内容/行为检查；不默认跑前端、生成器或保存回放 |
| Rust 规则 / importer | 新行为与真实受影响调用路径的聚焦测试、相关格式和 lint；相同目标已由 test 编译时不重复 check |
| 界面 / 输入 / 投影消费者 | 对应测试文件和 typecheck；构建链或产物是主题时再 build |
| 内容类型 / 协议 | 相应生成物检查与消费者验证，不重生成其他无变化格式 |
| 持久状态 / RNG / 公共初始化 / 共享投影 | 对应保存恢复、回放和受影响契约；范围横跨公共行为时才扩大 |

测试名称优先完整路径或明确模块名。短词可能误匹配无关用例；例如历史 `ogre` 过滤曾匹配到含 `progression` 的名称。单个测试可加 `-- --exact`，同一行为族用模块路径。

```powershell
# 仓库根目录：示例为能力的物品行为族，按本批替换
cargo test -p rfb-core --lib game::tests::abilities::items::
cargo clippy -p rfb-core --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
```

以上命令是用法示例，不是每批必跑流水线。Clippy/check 的 crate 和 targets 按真实改动选择；不为纯文档跑 Clippy。前端在 `web` 选择对应测试，如 `node --test src/character-traits-panel.test.ts`，再按需要运行 `npm run typecheck`。全量 `npm test` 留给实际跨界面改动或明确验收。

## 宠物 standalone 验收

在 `web` 执行 `npm run build:standalone:debug`，随后执行 `node e2e/pets-standalone.e2e.mjs`。脚本启动本次普通 Tauri EXE，以独立 WebView 配置正常创建人类战士；调用 ignored 的 `export_pet_desktop_save` 测试准备局部照明地形、受控马／大巫妖、敌方魔像及宠物携物，保留真实馆藏绑定。准备选择使第一次等待发生正式 AI 召唤的 RNG，未使用 WebDriver 专用准备 IPC。

通过正式菜单与原生键盘执行命名、权限切换、双手控骑、指定目标和召唤，再进入世界地图、移动、返回局部地图和解散坐骑；确认取消、窄窗口菜单以及每步原生保存／载入后的完整状态哈希。世界地图转换另断言原宠物、召唤来源与坐骑保留。报告、准备说明、存档和截图在 `test-results/pets/`；核心命令预测来自同一正式规则，不能仅以哈希相等代替操作语义断言。繁殖负担、施法安全、失控／死亡及无空间切换沿核心专项覆盖。准备场景不代表自然取得宠物、自然练级或 Android 验收。

## 桌面 E2E 快速到地牢

职业实战不需要验收城镇步行时，在 `web` 执行下列命令。先运行一次 `npm run e2e:build`；修改 Rust 后需重新构建。

```powershell
node e2e/tauri.e2e.mjs --ranger-play --fast-entry
node e2e/tauri.e2e.mjs --priest-play --fast-entry
node e2e/tauri.e2e.mjs --warrior-mage-play --fast-entry
node e2e/tauri.e2e.mjs --mage-play --fast-entry
node e2e/tauri.e2e.mjs --duelist-ui --fast-entry
node e2e/tauri.e2e.mjs --berserker --fast-entry
```

`--fast-entry` 复用 [dungeon-entry.e2e.mjs](../web/e2e/dungeon-entry.e2e.mjs)：从核心快照找到当前地图的实际下楼梯，经 `prepare_stairs_e2e({ position })` 将角色放到入口，再由原脚本点击下楼。该命令只在 WebDriver 构建可用，普通 EXE 拒绝调用；不授予经验/装备、不推进回合，也不预造地牢。地牢生成、怪物、战斗和后续 RNG 仍走正式规则；跳过路上的行动会使后续 RNG 起点不同于完整步行。

准备后的状态通过正常存档读取路径进入 UI，报告记录位置、前后哈希、耗时和跳过步行的说明。去掉 `--fast-entry` 即恢复完整步行；验收移动/城镇路途时保留默认流程。现行这些脚本以兽穴为目标，不把该参数用于任意世界传送。

新增实战脚本可在正式创角后调用 `await prepareDungeonEntry(driver)`，随后沿正常 UI 执行下楼；不要再复制城镇寻路循环。

## 食魔者桌面验收

在`web`执行`npm run e2e:build`，随后执行`node e2e/tauri.e2e.mjs --magic-eater-ui`。场景默认复用快速楼梯入口，正常下楼生成兽穴；无需额外的`--fast-entry`参数。先正常人类1级出生、吸收出生魔杖并攻击自然怪物，再明确准备25级、前哨站照明清场、30个生成/吸收装置和脚下替换物。每类首槽先准备一次使用的SP，实际执行耗尽、普通休息充满和再次使用；从正常导出存档加载后重演，比较完整事件、槽位投影和状态哈希。

中英文流程包含满槽覆盖/铭刻继承、换位标签、详情、行走设置、待选择/已确认存档、键盘取消与忙碌锁。390px和200%只用于专项截图，之后及退出时恢复1280×720、100%缩放。报告、截图与存档在`test-results/magic-eater-ui/`。自动消费者、旅店/两塔和下一次生成/SP小数/RNG由核心用例验证，桌面准备不代表自然取得30件装置或自然练级。

可玩优化产物使用`npm run build -- --no-bundle`。在根目录执行`web/e2e/mage-optimized.e2e.ps1 -Executable <EXE绝对路径> -OutputDirectory <证据目录> -Class MagicEater`，通过进程定向UI Automation验证普通无领域创角、1级/无公共MP、体内菜单及正常退出；此原生烟测与WebDriver实战分别记录。与其他任务共用桌面时先协调占用，再启动E2E；本流程不包含Android验收。

## 两城地图桌面验收

在 `web` 先执行 `npm run e2e:build`，再执行 `node e2e/tauri.e2e.mjs --town-maps`。使用 Tauri 专用 WebDriver 构建，独立应用标识 `io.github.unclefvcker.rfb-rewrite.e2e` 隔离日常存档和共享馆藏；地图不是浏览器模拟数据。

[场景脚本](../web/e2e/town-maps.e2e.mjs)从正常人类战士创角开始。WebDriver 准备接口仅负责到访阿南巴／萨洛斯并揭示当前地表，既有补给接口放置 10000 测试金币，再由原生拾取键取得。普通 EXE 拒绝准备接口。没有修改地形、授予经验、预设任务结果或清除沿途怪物；不把这段准备称为自然抵达城镇。

后续逐步发送原生数字键移动，覆盖城区商店、阿南巴双门共享库存、城门和远端任务入口；通过 UI 购买、丢物、接取／进入／放弃任务、世界地图往返、旅店往返，以及每城三次原生保存／加载。断言跨荒野滚动后的坐标、完整保存哈希和往返后的地面物品身份。`test-results/town-maps/report.json` 记录路线、视图偏移及画布诊断，同目录保存城区／城门／入口／返回画面；失败细节使用既有 `test-results` 日志和截图。该场景不代替任务成功结算与条件设施的核心专项，也不代表自然练级通关或 Android 验收。

## 祖尔桌面验收

在 `web` 执行 `npm run e2e:build`，随后执行 `node e2e/tauri.e2e.mjs --zul`。[祖尔场景](../web/e2e/zul.e2e.mjs)复用两城脚本的原生键盘、滚动坐标、截图和保存／加载助手，使用同一独立 WebDriver 应用标识。普通可玩产物另用 `npm run build:standalone:debug`，输出 `target/debug/rfb-tauri.exe`。

场景从兽化人、巫术／自然法师正常创角开始；该组合仅用于三塔身份与副领域切换，后续通用路线／战斗验收使用近战角色。准备接口物理到访并揭示祖尔，授予 50 级、一册生命书、长时浮空／无敌、八项非零美德和地面测试金币；玩家用拾取键取得金币。首次等级准备会清除当前怪物，后续祖尔继续按正式荒野规则刷怪；路线每段开始按核心实际实体数（包含视野外怪物）检查，发现实体时使用同一专用清怪入口，记录数量与位置。路线起点和门口使用原生键盘，长距离按当前地图预先规划最多 16 步，顺序调用正式 Rust 移动命令；卷屏或实际落点偏离预期即停止该段，段末原生保存恢复并核对完整状态哈希，不逐格等待界面重绘。四个任务分别先进入、截图和原生保存，再显式移除敌人及其携带物，保留地图地形和源地面物，由正常等待触发目标检查。普通 EXE 拒绝准备接口。

实际 UI 流程覆盖普通／珠宝／龙皮购物、丢物、三塔身份和服务、生命／自然副领域切换、两个方向的视野滚动、四图进出与源奖励、巫术塔和旧城旅店往返。荒野位置／视图偏移来自核心专用检查响应，不把边缘攻击的滚动误算成移动。原生保存／加载逐次核对完整状态哈希；最终往返核对巫术塔落点及原地物品身份。报告和截图在 `test-results/zul/`；失败细节沿用 `test-results` 诊断。完整流程通过后可执行 `node e2e/tauri.e2e.mjs --zul --zul-map-review`：读取四个任务内原生检查点，显式清场／揭示并用 45% WebView 缩放查看完整地图，另写 `map-review-report.json`，结束时恢复最终跨城检查点及原缩放，不覆盖主流程报告。整图落在视窗内仅是这四张截图的完整性检查，不限制正常游戏地图尺寸或镜头滚动。此模式显式刷新一次原生存档列表，并用后端日志断言新游戏和各次选中槽位加载不触发额外列表扫描。无敌和任务清场属于显式测试准备，不代表自然战斗、练级通关、Chaos 施法或 Android 验收。祖尔的失败／放弃、价格拒绝、地形伤害和来源随机边界由核心专项覆盖，见[计划 Z6](../design/zul-town-import-plan-20260912.md#z6聚焦桌面验收与来源收口)。

## 至尊魔戒读取桌面验收

在 `web` 执行 `npm run e2e:build`，随后执行 `node e2e/tauri.e2e.mjs --one-ring`。[聚焦脚本](../web/e2e/one-ring.e2e.mjs)正常创建人类1级战士，再通过核心导出测试显式选择出生天赋、清怪、移到准备地格并授予零充能魔戒，保留真实博物馆绑定。准备存档经过正常加载入口后，由UI选择、取消、读取、丢到脚下再读取；核对四行中文消息、物品不消耗及原生保存恢复后的相同下一次读取哈希。报告、存档与两张截图在 `test-results/one-ring/`。该场景只验收读取交互；普通生成、装备战斗、激活、时间／状态边界沿核心戒指专项验证，不宣称自然获取或Android验收。

## 辛葛的装置间充能桌面验收

在 `web` 执行 `npm run e2e:build`，随后执行 `node e2e/tauri.e2e.mjs --thingol`。[聚焦脚本](../web/e2e/thingol.e2e.mjs)正常创建人类1级战士，核心导出测试显式选择出生天赋、清怪、授予并装备辛葛的披风、提供背包供能装置及脚下零能量装置，并选择一次成功且供能未损毁的种子。UI验证两个独立选择框、任一阶段取消的时间／能量边界、背包到脚下充能、冷却存档和原生加载后的同次充能哈希。报告、存档及截图在 `test-results/thingol/`。供能损毁／目标失败、普通生成与700 tick边界由核心专项覆盖；不宣称自然获取、练级或Android验收。

## 阿斯加德验收准备

AS7 已完成来源与生成物收口，并执行核心、阿斯加德桌面及普通 EXE 检查。实际结果、途中修复和检查点续跑范围见[计划 AS7](../design/asgard-dungeon-plan-20260912.md#as7来源状态与交付收口)，以下命令用于复现。

仓库根目录先执行核心专项与普通 Tauri 准备接口拒绝分支；AS2–AS5 其余相关回归和全局契约范围见[计划](../design/asgard-dungeon-plan-20260912.md)。

```powershell
cargo test -p rfb-core --lib game::tests::asgard::
cargo test -p rfb-tauri --lib tests::ordinary_native_app_rejects_asgard_preparation_before_session_access -- --exact
```

在 `web` 执行专用桌面场景，并用正常 standalone 构建产物检查真实 IPC 拒绝：

```powershell
npm run e2e:build
node e2e/tauri.e2e.mjs --asgard
npm run build:standalone:debug
node e2e/asgard-standalone.e2e.mjs
```

[核心流程](../crates/rfb-core/src/game/tests/asgard/acceptance.rs)和[桌面场景](../web/e2e/asgard.e2e.mjs)共用受限的 [Rust 准备](../crates/rfb-core/src/game/floor/asgard_e2e.rs)：正常新战士出生选出北欧激活的种子，物理放到 (94,11)，给予 50 级、+100／+100 阔剑、两张召回之语卷轴、临时 +2000 最大 HP、+1000 近战技能／伤害、满有效玩家 HP 和 200000 tick 浮空／无敌／看见隐形，免疫流血／失明／混乱／恐惧／麻痹／震慑，揭示地图／隐藏门／陷阱。沿途清场按实际实体列表，保留海姆达尔、奥丁、维达及其携带物、能量和状态；战前仅将玩家放到既有目标旁并补满玩家 HP，不改三者 HP／最大 HP、种类或定义。三场战斗由生产攻击命令结算；这是准备后的攻击／流程验证，不是自然练级／难度通关；敌方仍使用正常 AI，玩家受伤、目标离开相邻位置或出现无关召唤时再次记录准备，攻击段在这些边界停止。独立核心用例另去掉玩家保护并检查三者源近战消费者。

场景覆盖正式入口、九深度路线、64／76／80／88 画面、64／80 正常镜头滚动、三者死亡、奥丁死后维达仍存活的原生检查点、阔刃长矛『符文长矛』与获得物品卷轴的拾取／详情／实际用卷轴、返回和召回。原生键盘处理门、楼梯、拾取及路线／战斗起点；长段最多 16 次生产 Rust 命令，目标死亡或位移异常即停止，段末经真实保存恢复同步 UI。每次准备写明前后哈希、玩家参数、移除与保留实体；每次选中原生加载断言没有全列表读取。普通镜头维持 100%，不要求整张 96×33 地图装入视窗。

桌面报告、19 张截图及失败记录在 `test-results/asgard/`。`route-report.json` 保留入口到战斗／奖励／地表返程的通过记录及旧逐回合召回段的同步超时；`recall-report.json` 从同一真实 `surface-return.rfbsave` 恢复，以原生 `r`／Rest 完成双向召回及四次原生保存恢复。`acceptance-summary.json` 汇总两段的对应哈希、范围和实际结果，不把续跑写成一次无中断全程通过。完整脚本现使用 Rest 等待召回，遇敌等中断后才重新准备；核心也验证满 HP 时待召回仍能休息推进。

`standalone-guard-report.json` 由[普通产物脚本](../web/e2e/asgard-standalone.e2e.mjs)启动 `target/debug/rfb-tauri.exe` 后生成，记录 EXE SHA-256 及 arrival／route／battle 三阶段的真实拒绝，全部通过。该脚本通过 WebView2 的 `--edge-webview-switches` 为本次进程开启本机 CDP，使用新的 WebView 配置目录，不创建游戏或原生存档；管理员进程会忽略环境变量里的调试参数，见 [Microsoft 文档](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security)。专用 E2E 仍使用独立应用标识；普通可玩 EXE 不能用 Cargo build 代替 Tauri standalone 构建。

按用户要求，AS7 不再重跑祖尔整流程；此前 Z6 的验收记录继续保留。共用助手在本批由实际使用它们的阿斯加德场景验证，不将已经通过的其他城镇整流程自动加入验收范围。

## 常规装备底材桌面验收

N3使用 `node web/e2e/ordinary-equipment-standalone.e2e.mjs --non-quest-n3`。复用同一普通Tauri与隔离目录，ignored导出从正常Mage新档准备50级、属性潜力、满HP、火把、无敌和睡眠战熊。四件代表分别验证追加时间伤害、弩发射普通箭、罪恶巨锤激活及金币消耗、丘比特魅惑；每步正常保存恢复并核对哈希。准备失败种子被排除，证据目录`test-results/non-quest-n3/`；不是自然练级、取得或完整源规则一致的证明。

N2使用 `node web/e2e/ordinary-equipment-standalone.e2e.mjs --non-quest-n2`。正常Mage新档经ignored导出用例准备50级、局部地格、无敌、睡眠目标／水格及真实普通或instant生成的五件神器；UI逐件拾取、装备、激活、等待，每步正常保存恢复并核对哈希。太公望激活前通过CDP暂停浏览器计时器，以固定自动钓鱼之间的保存检查点，后续手动等待仍执行正式核心命令；自动继续钓鱼另有输入控制器覆盖。证据目录`test-results/non-quest-n2/`，仍使用下述隔离构建目录；不代表自然取得、练级或全部源规则一致。

N1使用 `node web/e2e/ordinary-equipment-standalone.e2e.mjs --non-quest-n1`。普通Tauri新建人类Mage后，ignored核心导出用例准备50级、局部地格、无敌及通过真实底材／稀有度生成的四件神器；Hellfire另配普通弩矢／火把，两件武器保留相邻源怪物并选击杀种子。UI拾取、装备、近战／射击／等待，14次操作每次保存恢复并核对哈希，证据在`test-results/non-quest-n1/`。不是自然获取或练级证明。脚本尊重`CARGO_TARGET_DIR`，本次产物在`target/n1-validation/debug/rfb-tauri.exe`；构建和导出共用隔离的`CARGO_BUILD_BUILD_DIR=.../target/n1-validation-build`，避免共享工作树缓存。Windows上先结束同一路径的测试EXE，再重链接或执行导出。

Q1命名奖励复用下述脚本的 `--quest-items` 模式：`node web/e2e/ordinary-equipment-standalone.e2e.mjs --quest-items`（仓库根目录）。它从正常1级人类战士出生存档准备局部场地、相邻当前HP1的Fang及真实击杀／掉落种子；物品由实际死亡掉落产生。UI攻击、移动、拾取、装备及等待，每步原生保存恢复并核对核心哈希。已有报告在`test-results/quest-items-q1/`。Q5实现结束后统一执行此前延后的编译和验收。

Q2–Q5代表流程使用 `node web/e2e/ordinary-equipment-standalone.e2e.mjs --quest-items-all`。从正常新档显式准备50级、天赋与无敌；眼球／九头蛇之眼／罗摩由相邻源怪物的真实死亡掉落产生，刺针由接受金库任务、进入正式地图产生。之后清理场景并定位真实奖励，罗摩另给10支普通箭；UI拾取、装备、激活／射击与等待，每步保存恢复。准备逻辑在ignored核心导出测试中，普通产物没有测试准备IPC。证据在 `test-results/quest-items-q2-q5/`；这不代表自然练级、战斗难度、全部神祇的桌面遭遇或Q4完整任务路线验收。

在 `web` 执行 `npm run build:standalone:debug`，随后执行 `node e2e/ordinary-equipment-standalone.e2e.mjs`。[场景](../web/e2e/ordinary-equipment-standalone.e2e.mjs)使用普通 Tauri EXE、隔离的 WebView 配置目录和正式创角／装备／输入／保存加载路径，不启用 WebDriver 专用准备 IPC。

正常创建人类1级战士并导出存档后，[核心导出用例](../crates/rfb-core/src/game/tests/death_scythe.rs)显式选择出生天赋、清怪、准备小片地格与相邻目标／岩浆矿脉，授予并鉴定七件代表底材。镰刀准备合法的 −255 命中附魔、临时 +2000 最大 HP、满有效 HP，并选择非致死反噬种子；钩镰枪及泳装目标起始睡眠。UI依次装备钩镰枪、矮人镐、秘银板甲／空灵披风／秘银护手、泳装和镰刀，并用原生按键攻击、挖掘或等待。每个动作后导出／正常加载，核对核心预演的完整状态哈希，再继续下一动作。

报告、截图和存档在 `test-results/ordinary-equipment/`。准备后的交互与保存证据不代表自然取得七件稀有装备或自然练级；完整源分配、其他成员、概率与致死边界由核心验证。普通镰刀不自动返回，共用返回反噬仅在核心准备的返回状态中验证；未开放的返回能力入口和 Monster Sword 内部 kind111 不在桌面范围内。

## 混沌领域桌面验收

在`web`运行`npm run build:standalone:debug`，随后从仓库根执行`node web/e2e/ordinary-equipment-standalone.e2e.mjs --chaos`。构建、导出和桌面脚本共用`CARGO_TARGET_DIR=.../target/n1-validation`与`CARGO_BUILD_BUILD_DIR=.../target/n1-validation-build`。普通产物不启用WebDriver准备IPC；ignored核心导出仅准备合法等级、书本、目标和种子，产品UI完成学习、施法、选物、追加方向及等待，27步各经原生保存恢复核对哈希。中英文25入口和8个场景见`test-results/chaos/report.json`，来源差异与核心验收见[混沌计划](chaos-realm-plan.md#ch5开放与统一验收2026-09-14)。这不是自然练级、自然获取或Android验收。

## 王牌领域桌面验收

王牌领域使用 `node web/e2e/ordinary-equipment-standalone.e2e.mjs --trump`。共享应用数据中的博物馆档案使用了当前分支不识别的格式，本次使用独立应用标识的普通standalone，保留原档案。构建及脚本设置 `CARGO_TARGET_DIR=.../target/trump-desktop-validation`、`CARGO_BUILD_BUILD_DIR=.../target/n1-validation-build`；在 `target/trump-standalone.conf.json` 写入 `{"identifier":"org.rfb.rewrite.trump-validation"}`，于 `web` 执行 `npm run build:standalone:debug -- --config ../target/trump-standalone.conf.json`。它只覆盖应用标识，未启用WebDriver或准备IPC。27 个王牌入口逐一检查中英文选择；六个场景共15步，覆盖第一册学习、相位门、召唤地点、恋人牌方向与取消、治疗宠物和烙印，每步原生保存恢复核对哈希。ignored 导出用例从正式高阶法师王牌构筑准备50级、满资源、无敌、书本、局部目标及实际施法成功种子。来源范围与限制见[王牌领域](trump-realm.md)，报告和截图在 `test-results/trump/`；不是自然练级或自然取得证明。

## 死灵领域桌面验收

在 `target/necromancy-standalone.conf.json` 写入 `{"identifier":"org.rfb.rewrite.necromancy-validation"}`，设置 `CARGO_TARGET_DIR=.../target/necromancy-desktop-validation` 与 `CARGO_BUILD_BUILD_DIR=.../target/n1-validation-build`，在 `web` 执行 `npm run build:standalone:debug -- --config ../target/necromancy-standalone.conf.json`，随后从根目录执行 `node web/e2e/ordinary-equipment-standalone.e2e.mjs --necromancy`。独立应用标识保留主线博物馆档案，普通产物没有准备IPC。

场景从正式死灵法师构筑准备50级、满资源、书本、局部地形、目标、无敌和实际成功／失败种子。中英文创建入口、第一册学习与触摸、成功宠物、失败敌对召唤、鉴定和护盾经产品UI操作，每步原生保存恢复核对哈希。报告在 `test-results/necromancy/`；来源、核心证据和适配见[死灵领域](necromancy-realm.md)。这不是自然练级、自然获取四册或Android验收。

## 随机荒野地牢桌面专项

在 `web` 执行：

```powershell
npm run e2e:build
node e2e/tauri.e2e.mjs --random-dungeons
npm run build:standalone:debug
node e2e/asgard-standalone.e2e.mjs --random-dungeons
```

专项复用原生键盘、保存列表与截图助手，四次正常人类战士创角后使用受限准备抵达正式生成的入口。出生种子为 42；森林／火山／山脉／海洋的已验证荒野种子分别为 1／6／301／0。准备给一张归返卷轴、长效悬浮／无敌并清场照明；上楼准备只定位已有楼梯。四类进入和深度、原生保存加载、上楼返回、再次入场新实例、召回返回均走正常命令；召回以 `rest` 推进。这不代表自然遭遇频率、练级、获取卷轴或战斗难度验收。

证据保存在 `test-results/random-dungeons/`。失败后可用 `--random-resume=forest:entered` 之类的参数恢复该目录的原生存档检查点；类型为 forest／volcano／mountain／sea，阶段为 arrival／entered／ascended／reentered／recall-pending／returned。脚本核对内容身份和保存哈希，恢复后只继续未完成阶段；改变内容后应重新生成相应检查点。普通 standalone 分支实际检查三阶段准备 IPC 均被拒绝，并要求窗口正常关闭后进程退出码为 0。

## 物品选择普通 standalone

在 `web` 执行 `npm run build:standalone:debug`，再执行 `node e2e/item-selection-standalone.e2e.mjs`。脚本启动普通 EXE，通过 WebView 原生键盘、界面按钮与生产存读档入口验证选择流程，不使用 WebDriver 专用 IPC。

正常创角后，脚本显式运行 `game::tests::item_selection_desktop::export_item_selection_desktop_save` 准备分页、四来源与铭刻所需物品，并按 Core 命令导出预期哈希。准备内容、报告、截图与原生存档写入 `test-results/item-selection/`；这不是自然取得全部物品的验收。场景检查两套预设、鼠标、分页／标签／来源、确认和取消、次数与重复、连续行动中断、旧窗口读档失效及中英文窄窗口。其他能力调用者继续由对应单元测试覆盖。

## 地图情报、帮助知识与配置记录普通 standalone

地图与情报的普通桌面专项：在 `web` 构建 `npm run build:standalone:debug` 后运行 `node e2e/map-intelligence-standalone.e2e.mjs`。使用正常新建的人类战士，选择已有出生天赋，不准备怪物、不揭图；键盘／按钮查询与原生载入前后比较状态哈希。证据在 `test-results/map-intelligence/`。地下城评分、友方／物品资格和已调查资料的保存往返由 `cargo test -p rfb-core --lib map_intelligence` 覆盖；桌面出生城镇场景不冒称已验收所有深度或怪物图鉴。

同一脚本覆盖帮助／知识入口：正常键盘打开 `?` 和 `~`，搜索命中与无结果、全部 22 项大小写菜单选择、Enter 打开已有资料页、Esc 逐层返回、同名工具栏按钮、读档关闭旧菜单及英文 390px 布局。全程查询与返回必须保持状态哈希一致。前端 `input-controller.test.ts` 覆盖五套预设、世界地图、编辑／IME／修饰键／弹窗保护和先停止连续行动；`help-knowledge.test.ts` 检查动态本地化键与档案分类、击杀排序。

持久发现档案通过 `web/e2e/discovery.e2e.mjs` 接入同一脚本：先保留正常新局的原生存档，再由显式 ignored 测试 `game::tests::discovery::export_discovery_desktop_save`（环境变量 `DISCOVERY_INPUT` 指向该存档）准备兽穴第 3 层、两只已认识唯一怪物、归属玩家的 Fang 死亡、调查资料及已鉴定神器／Ego。随后移除相关物品与局部怪物，保留原馆藏绑定，输出 `discovery-prepared.rfbsave` 和 `discovery-scenario.json`。普通 EXE 经正式载入入口验证七类档案、存活／死亡筛选、击杀数、最深层数、窄屏、只读哈希及保存恢复；最后恢复原正常角色。此准备验证记忆持久性，不代表自然取得神器或自然通关。核心 `game::tests::discovery::` 覆盖隐藏身份、鉴定门槛、死亡归属、唯一怪物占用与死亡区分、深度及非法存档；详细属性仍沿用调查知识，不宣称逐次战斗的全部原版 lore 抽样。

配置／记录通过同一脚本调用 `web/e2e/config-records.e2e.mjs`：捕获 F2、自定义映射与反斜杠绕过、预设隔离、命令菜单真实行动、单命令寄存器、录制／编辑／回放多步宏、键位 JSON 导出导入及寄存器热键、笔记文本、当前可见地图 PNG／TXT／离线 HTML 导出。下载通过 WebView 正常链接生成真实文件，保存在本次 `test-results/map-intelligence/exports-*` 子目录；无测试专用导出 IPC。原生读档应保留本机键位和笔记、清除会话录制并关闭旧窗口，最后检查英文 390px 配置页。`command-recording.test.ts` 与输入／会话测试覆盖深拷贝、录制上限、取消与失败、过期参数和修饰键；截图迷雾边界沿用 Core 投影与渲染，PNG 需另做实际图像检查。
## 安格班桌面专项

在 `web` 执行：

```powershell
npm run e2e:build
node e2e/tauri.e2e.mjs --angband
node e2e/tauri.e2e.mjs --angband --angband-resume=completed-54
npm run build:standalone:debug
node e2e/asgard-standalone.e2e.mjs --angband
```

续跑参数取 `test-results/angband/` 内已有检查点名，不含扩展名。包括入口、每个随机任务前后、首次撤退／重入、`oberon-before`、`serpent-before`、`victory`、`depth-101`、`depth-127`、`returned`、`retired`；脚本核对内容身份、协议及完整保存哈希。普通产物脚本会拒绝 arrival／route／battle／stairs-up／stairs-down 五种准备，再用正常人类战士创角进入地图并关闭窗口，要求退出码 0。

专项从正常 1 级人类战士开始，明确准备正式 (57,40) 入口位置、51 级所需经验（胜前仍封顶 50）、阔剑、归返卷轴、长效悬浮／识破隐形／无敌、额外 HP 与近战能力、状态免疫。后续只定位实际楼梯或相邻战斗格、补玩家 HP、照明及清理非任务目标。十个随机目标、奥伯龙和混沌之蛇保持源 HP、防御和 AI，通过生产命令击杀；不写任务完成、掉落或胜利状态。每场首击走原生键盘，其余攻击保留命令事件，每最多 16 击及死亡后用真实保存加载同步 UI。入口、任务层及 99／100／101／127 深度边界走原生楼梯输入；其余中间层执行生产楼梯命令并记录事件，到下一任务层恢复同步界面。任务层下行限制、首次撤退重入、胜后深层探索、召回和隐退均保留证据。隐退按钮的取消／确认由测试控制浏览器确认回调，原生系统确认框外观不在该自动检查范围内。

召回首轮 Rest 走原生键盘，其余轮使用生产 Rest 命令并记录事件，结束后才重新加载界面；避免每个饥饿打断回合都往返传输约 10 MB 的深层存档。

这是明确准备后的流程验收，不代表自然练级、装备获取或难度通关。报告、截图、原生存档与普通产物 SHA-256 位于上述目录；实际批次结果见[安格班计划](../design/angband-dungeon-plan-20260913.md)。

## Contract fixture

当前集位于 [tests/fixtures/active/scenarios](../tests/fixtures/active/scenarios/)，分类和最低数量等政策来自 [baseline-policy.json](../tests/fixtures/active/baseline-policy.json)。`rfb-contract` 的 [CLI](../crates/rfb-contract/src/main.rs)和[断言实现](../crates/rfb-contract/src/lib.rs)是精确语义依据。

- 一条 fixture 聚焦一个行为；不为购买测试走完整段路、不组合多个设施访问。
- 非移动主题直接设玩家位置。通用购买选择第一项投影库存，物品身份本身是主题时才固定实例。
- `assertions.finalState` 只填写场景需要的字段，但保留完整 stateHash。对象按已填字段递归比较；空对象表示集合为空；数组的内容与顺序精确比较。
- 事件、错误与保存回环哈希保持精确断言。不要删关键断言、加 waiver 或刷新整套数据掩盖失败。

在根目录使用本批真实 fixture 路径：

```text
cargo run -p rfb-contract -- observe <fixture.json>
cargo run -p rfb-contract -- verify <fixture.json>
cargo run -p rfb-contract -- refresh <fixture.json>
```

`observe` 用于看实际结果；只有解释了行为差异后才执行 `refresh`。它保留已有 finalState 的字段选择，不代表差异自动合理。新增场景可从完整观察中裁出最小断言。

分类验证可避免全量回放：

```powershell
cargo run -p rfb-contract -- list-categories tests/fixtures/active/baseline-policy.json
cargo run -p rfb-contract -- verify-category tests/fixtures/active/baseline-policy.json town
```

根据实际类别使用 `refresh-category`。没有行为变化就复验或复用有效结果，不刷新预期。

## 何时全量，何时改版本

`contentHash` 不参与状态哈希。内容版本/hash 变更本身不要求升级 State Hash Schema 或全量刷新；内容若改变共同出生地图、全局候选池或 RNG，仍按实际行为影响处理。

协议版本、save header/payload、保存容器、State Hash Schema 和内容包版本分别由对应模型控制。只有格式或行为契约实际变化才调整相应版本；类型名中的历史版本后缀不能作为升级依据。

`verify-all`、`refresh-all` 与 ignored 的完整 fixture 回放用于公共状态哈希输入、共享协议投影、公共初始化/RNG 变化或明确里程碑。普通 merge 不自动升级为完整验收。集成可复用方向有效的测试记录，补查合并引入的交叉影响与冲突修复。

## 明确的集成里程碑

确定需要整轮验收时，在根目录使用：

```powershell
cargo test --workspace --exclude rfb-tauri
cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings
cargo check -p rfb-tauri --all-targets
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo test -p rfb-contract --test contract_fixtures committed_contract_fixtures_pass -- --ignored --exact
```

前端在 `web` 执行 `npm test`、`npm run build:ui`。桌面 E2E 只有相关问题或明确玩家流程验收时再执行 `npm run e2e`；它生成独立的 WebDriver 调试构建。渲染实验另用 `npm run e2e:render-profile`，Android 单独选取，不附带运行。

CI 在 [.github/workflows/ci.yml](../.github/workflows/ci.yml) 和 [windows.yml](../.github/workflows/windows.yml) 配置自己的触发范围；本指南没有修改 CI，也不要求每次本地编辑重复它的整套工作。

## 角色结束与高分榜验收

在 `web` 执行 `npm run build:standalone:debug`，随后执行 `node e2e/character-ending-standalone.e2e.mjs`。脚本使用普通 EXE、独立 WebView 配置，沿正式创角流程创建同名同种子人类战士；角色和成绩仍写入该应用的本机档案，保留原有记录。无需调试准备 IPC 或修改角色存档。原生键盘验证 Q 的取消、提示框取消、错误口令与 `@` 确认；核对结束页、权威分数、390px 横向表格、旧活动存档恢复最终检查点、禁止继续行动、前端重载后保留成绩、另一个角色独立入榜，以及标题／结束页和 `~ H` 入口。

报告及桌面／窄窗口截图位于 `test-results/character-ending/`。原生测试 `score_storage_tests` 补足实际死亡、写入失败后重试、同一角色陈旧会话、进程状态重建及结束命令回放；既有馆藏跨进程锁与中断恢复测试随原生层回归。核心 `game::tests::ending` 验证局部／世界地图结束的零时间及终态保存，`warrens_dungeon_conquest_returns_retires_and_round_trips` 同时覆盖原有地表退休与胜利后地牢内立即退休。桌面角色未自然通关，不将其称为死亡／胜利全过程的桌面验收；本批未验收 Android。

## 记录结果

记录提交、检查命令/范围、通过或失败和相关限制即可。代码审查、自动测试、桌面操作与人工试玩分别记录。历史通过结果只证明当时的代码和范围，不自动成为当前提交的新证据。

## 全局偏好 O8 验收

先构建普通 Tauri standalone，再从 `web/` 执行：

```powershell
npm run build:standalone:debug
node e2e/global-preferences-standalone.e2e.mjs
```

仅在没有其他游戏实例写入偏好时运行。脚本备份应用数据目录中的原始 `preferences.json`，使用全新默认偏好和独立 WebView 调试目录，最后恢复原始字节（原文件不存在则移除测试创建的文件）。不会删除角色存档、成绩或馆藏。场景正常创建两个人类战士，测试两套 RFB 键位选择、与物品栏相同的设置／宠物窗口、分类导航、地牢信息常显、标题设置、跨进程／角色继承、字形与配色、JSON／PRF 导入导出、草案取消、`$` 与编辑器重载和英文 390px；没有专用角色准备 IPC。报告、下载文件和截图在 `test-results/global-preferences/`。

规则侧运行 Core 与原生库、协议／保存／回放测试；行为上下文和保存边界有变化时显式验证 26 条 active 契约。完整命令结果、解释过的 fixture 差异和桌面范围见 [O8 验证记录](global-preferences-plan.md#o8-统一验证记录2026-09-15)。
