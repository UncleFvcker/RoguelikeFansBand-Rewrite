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

## 两城地图桌面验收

在 `web` 先执行 `npm run e2e:build`，再执行 `node e2e/tauri.e2e.mjs --town-maps`。使用 Tauri 专用 WebDriver 构建，独立应用标识 `io.github.unclefvcker.rfb-rewrite.e2e` 隔离日常存档和共享馆藏；地图不是浏览器模拟数据。

[场景脚本](../web/e2e/town-maps.e2e.mjs)从正常人类战士创角开始。WebDriver 准备接口仅负责到访阿南巴／萨洛斯并揭示当前地表，既有补给接口放置 10000 测试金币，再由原生拾取键取得。普通 EXE 拒绝准备接口。没有修改地形、授予经验、预设任务结果或清除沿途怪物；不把这段准备称为自然抵达城镇。

后续逐步发送原生数字键移动，覆盖城区商店、阿南巴双门共享库存、城门和远端任务入口；通过 UI 购买、丢物、接取／进入／放弃任务、世界地图往返、旅店往返，以及每城三次原生保存／加载。断言跨荒野滚动后的坐标、完整保存哈希和往返后的地面物品身份。`test-results/town-maps/report.json` 记录路线、视图偏移及画布诊断，同目录保存城区／城门／入口／返回画面；失败细节使用既有 `test-results` 日志和截图。该场景不代替任务成功结算与条件设施的核心专项，也不代表自然练级通关或 Android 验收。

## 祖尔桌面验收

在 `web` 执行 `npm run e2e:build`，随后执行 `node e2e/tauri.e2e.mjs --zul`。[祖尔场景](../web/e2e/zul.e2e.mjs)复用两城脚本的原生键盘、滚动坐标、截图和保存／加载助手，使用同一独立 WebDriver 应用标识。普通可玩产物另用 `npm run build:standalone:debug`，输出 `target/debug/rfb-tauri.exe`。

场景从兽化人、巫术／自然法师正常创角开始；该组合仅用于三塔身份与副领域切换，后续通用路线／战斗验收使用近战角色。准备接口物理到访并揭示祖尔，授予 50 级、一册生命书、长时浮空／无敌、八项非零美德和地面测试金币；玩家用拾取键取得金币。首次等级准备会清除当前怪物，后续祖尔继续按正式荒野规则刷怪；路线每段开始按核心实际实体数（包含视野外怪物）检查，发现实体时使用同一专用清怪入口，记录数量与位置。路线起点和门口使用原生键盘，长距离按当前地图预先规划最多 16 步，顺序调用正式 Rust 移动命令；卷屏或实际落点偏离预期即停止该段，段末原生保存恢复并核对完整状态哈希，不逐格等待界面重绘。四个任务分别先进入、截图和原生保存，再显式移除敌人及其携带物，保留地图地形和源地面物，由正常等待触发目标检查。普通 EXE 拒绝准备接口。

实际 UI 流程覆盖普通／珠宝／龙皮购物、丢物、三塔身份和服务、生命／自然副领域切换、两个方向的视野滚动、四图进出与源奖励、巫术塔和旧城旅店往返。荒野位置／视图偏移来自核心专用检查响应，不把边缘攻击的滚动误算成移动。原生保存／加载逐次核对完整状态哈希；最终往返核对巫术塔落点及原地物品身份。报告和截图在 `test-results/zul/`；失败细节沿用 `test-results` 诊断。完整流程通过后可执行 `node e2e/tauri.e2e.mjs --zul --zul-map-review`：读取四个任务内原生检查点，显式清场／揭示并用 45% WebView 缩放查看完整地图，另写 `map-review-report.json`，结束时恢复最终跨城检查点及原缩放，不覆盖主流程报告。整图落在视窗内仅是这四张截图的完整性检查，不限制正常游戏地图尺寸或镜头滚动。此模式显式刷新一次原生存档列表，并用后端日志断言新游戏和各次选中槽位加载不触发额外列表扫描。无敌和任务清场属于显式测试准备，不代表自然战斗、练级通关、Chaos 施法或 Android 验收。祖尔的失败／放弃、价格拒绝、地形伤害和来源随机边界由核心专项覆盖，见[计划 Z6](../design/zul-town-import-plan-20260912.md#z6聚焦桌面验收与来源收口)。

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

## 记录结果

记录提交、检查命令/范围、通过或失败和相关限制即可。代码审查、自动测试、桌面操作与人工试玩分别记录。历史通过结果只证明当时的代码和范围，不自动成为当前提交的新证据。
