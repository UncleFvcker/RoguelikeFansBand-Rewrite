# Contract 基准更新政策

状态：active-only Policy v2 已由 `rfb-contract` 和 CI 强制验证

## 1. 目的

contract fixture 是规则兼容边界，不能把测试失败简单处理为“刷新预期结果”。政策用于保证每次规则变化只修改真正受影响的场景，同时保留可审查的失败原因。

当前逻辑基线是 `contract-v237`，机器可读政策固定在：

```text
tests/fixtures/active/baseline-policy.json
```

## 分类验证

每条 fixture 必须声明一个受控的主 `category`。分类表示该 fixture 主要保护的行为，不是自由标签；跨系统改动一次选择多个分类即可。当前分类可通过下列命令查询，输出同时给出各类数量：

```powershell
cargo run -p rfb-contract -- list-categories tests/fixtures/active/baseline-policy.json
```

日常规则改动只回放相关分类：

```powershell
cargo run -p rfb-contract -- verify-category tests/fixtures/active/baseline-policy.json inventory equipment
cargo run -p rfb-contract -- refresh-category tests/fixtures/active/baseline-policy.json inventory equipment
```

`refresh-category` 会先为选中分类计算全部断言；任一场景计算失败时不会写入该批文件。普通 `cargo test -p rfb-contract` 仍快速检查全部 fixture 的 JSON、schema、分类和 ID 唯一性，但不会运行 471 条完整回放。

只有以下变化默认需要全量回放或刷新：

- contract assertion 或公共 protocol 投影字段变化；
- state hash Schema 或所有场景都可观察到的 state hash 输入变化；
- 公共初始化、RNG、存档往返语义变化；
- 明确的里程碑验收。

对应命令为：

```powershell
cargo run -p rfb-contract -- verify-all tests/fixtures/active/baseline-policy.json
cargo run -p rfb-contract -- refresh-all tests/fixtures/active/baseline-policy.json
cargo test -p rfb-contract --test contract_fixtures committed_contract_fixtures_pass -- --ignored
```

`contentHash` 由存档与回放头独立做精确内容匹配，不属于 state hash
输入。纯内容 hash 更新本身不再要求刷新全部 fixture；只有场景实际观察到的
内容、初始化状态或行为发生变化时，才刷新对应分类。

工作树只保留这一份 active fixture 集。历史基线由 Git 提交、tag 或 release artifact 保存，不再复制为 `contract-vN` 目录。

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/active/baseline-policy.json
```

## 2. 存储规则

- `tests/fixtures/active/scenarios/` 是唯一提交到主分支的场景目录。
- contract 逻辑版本由 `rfb_contract::ACTIVE_BASELINE` 和 policy 的 `baseline` 字段共同声明。
- 升级 contract 时不复制或重命名 active 目录。
- 只新增新场景，或更新语义确实变化的 assertions。
- JSON 可以省略由 contract Schema 明确定义的默认值；反序列化后的完整对象才是 exact 比较边界。
- 历史结果从对应 Git 提交恢复，不在当前工作树重复保存。
- `active/waivers/` 只保留 `.gitkeep`。当前不接受 waiver 文件。
- fixture 应只覆盖一个最小行为。除专门验证移动外，位置相关场景使用 `playerPosition` 前置条件，不混入移动命令，也不在一个 fixture 中串联多个设施。
- 通用商店购买场景使用 `buy-first-from-shop` 选择当前投影库存首项；只有物品身份或物品自身行为是测试主题时，才绑定具体实例。

## 3. 更新流程

1. 运行失败的 fixture，保留原 assertions。
2. 使用 `observe` 查看实际结果并定位实现、工具或规则变化。
3. 实现错误和工具错误必须修复，不能刷新 fixture 掩盖。
4. 预期规则变化必须在对应 contract 文档中记录原因、事务顺序和玩家可见影响。
5. 只对受影响的 fixture 执行 `refresh`，然后人工审阅完整 diff。
6. 新场景加入 active 目录，并相应提高 policy 的 `minimumFixtureCount`。
7. 更新 `ACTIVE_BASELINE` 和 policy 的 `baseline`，执行 policy 与全部 exact fixture 验证。

禁止批量 refresh 未受影响场景，也禁止为了“让测试通过”降低最低 fixture 数量。仅改变默认字段是否落盘的全量重写属于表示迁移，必须在不调用 `observe` 的情况下完成，并证明重写前后的反序列化对象完全相等。

## 4. Policy v2

机器可读 policy 包含：

- `schemaVersion`：当前为 2；
- `baseline`：当前逻辑 contract 版本；
- `legacyCommit`：固定旧版参考 commit；
- `contractSchemaVersion`；
- `normalizationSchemaVersion`；
- `minimumFixtureCount`；
- `fixtureDirectory` 和 `waiverDirectory`：仅允许 policy 目录下的安全相对路径。

验证器会解析完整 fixture 集、检查最小数量和场景集合不变量，并拒绝 `waivers/` 中除 `.gitkeep` 外的任何条目。

如果将来出现无法通过普通规则修正处理的真实 waiver 需求，应以独立设计重新引入最小审批模型；不预先维护 issue、批准人、双 hash、过期日期等未被使用的公共格式。

## 5. 当前边界

`contract-v119` 接入可见亡灵驱散与逐目标放逐，协议 DTO 和 state hash Schema 均未改变。边界由 [Contract v119](contract-v119-scroll-visible-actor-effects.md) 定义。active 集包含 422 个 exact fixtures，零 waiver。
