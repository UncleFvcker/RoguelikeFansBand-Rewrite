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
