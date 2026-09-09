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
