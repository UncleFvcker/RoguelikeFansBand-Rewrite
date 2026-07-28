# Contract 基准更新政策

状态：active-only Policy v2 已由 `rfb-contract` 和 CI 强制验证

## 1. 目的

contract fixture 是规则兼容边界，不能把测试失败简单处理为“刷新预期结果”。政策用于保证每次规则变化只修改真正受影响的场景，同时保留可审查的失败原因。

当前逻辑基线是 `contract-v117`，机器可读政策固定在：

```text
tests/fixtures/active/baseline-policy.json
```

工作树只保留这一份 active fixture 集。历史基线由 Git 提交、tag 或 release artifact 保存，不再复制为 `contract-vN` 目录。

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/active/baseline-policy.json
```

## 2. 存储规则

- `tests/fixtures/active/scenarios/` 是唯一提交到主分支的场景目录。
- contract 逻辑版本由 `rfb_contract::ACTIVE_BASELINE` 和 policy 的 `baseline` 字段共同声明。
- 升级 contract 时不复制或重命名 active 目录。
- 只新增新场景，或更新语义确实变化的 assertions。
- 历史结果从对应 Git 提交恢复，不在当前工作树重复保存。
- `active/waivers/` 只保留 `.gitkeep`。当前不接受 waiver 文件。

## 3. 更新流程

1. 运行失败的 fixture，保留原 assertions。
2. 使用 `observe` 查看实际结果并定位实现、工具或规则变化。
3. 实现错误和工具错误必须修复，不能刷新 fixture 掩盖。
4. 预期规则变化必须在对应 contract 文档中记录原因、事务顺序和玩家可见影响。
5. 只对受影响的 fixture 执行 `refresh`，然后人工审阅完整 diff。
6. 新场景加入 active 目录，并相应提高 policy 的 `minimumFixtureCount`。
7. 更新 `ACTIVE_BASELINE` 和 policy 的 `baseline`，执行 policy 与全部 exact fixture 验证。

禁止批量 refresh 未受影响场景，也禁止为了“让测试通过”降低最低 fixture 数量。

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

`contract-v117` 的四种物品类别召唤、深度或玩家等级过滤、Race kin、永久玩家控制、零结果知识/RNG、save/replay 和 state hash Schema v52 由 [Contract v117](contract-v117-scroll-summoning.md) 定义。active 集包含 420 个 exact fixtures，零 waiver。
