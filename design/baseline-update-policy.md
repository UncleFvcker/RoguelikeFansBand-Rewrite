# Contract 基准更新与差异豁免政策

状态：Policy v1 已实现并由 `rfb-contract`/CI 强制验证

## 1. 目的

contract fixture 是规则兼容边界，不能把测试失败简单处理为“刷新预期结果”。本政策用于区分：

- 实现错误或无意回归；
- 规范化器或测试工具错误；
- 已讨论并批准的规则修正；
- 新核心明确不复刻的旧版行为；
- contract Schema 或 state hash Schema 的正式迁移。

`tests/fixtures/contract-v10/baseline-policy.json` 是当前 active 机器可读政策。`contract-v1` 至 `contract-v9` 保留为历史基准。公共 CI 每次运行：

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v10/baseline-policy.json
```

## 2. 禁止操作

- 禁止批量覆盖所有 fixture assertions；
- 禁止使用 `*`、`all` 或目录级范围绕过单个场景审查；
- 禁止只填写“更新测试”“适配新代码”等无法解释规则变化的原因；
- 禁止旧、新规范化 hash 相同的空豁免；
- 禁止没有 issue、批准人或批准日期的豁免；
- 禁止直接替换固定旧版 commit；更换基准必须新增 policy/fixture 版本；
- 禁止把本地旧存档、旧文本或旧素材附加到豁免文件。

## 3. 更新流程

1. 运行失败的 fixture，保留原 assertions；
2. 使用 `observe` 查看实际结果；
3. 使用快照规范化器分别计算旧、新结果 hash；
4. 判断差异属于 bug、工具问题还是预期规则变化；
5. bug 或工具问题必须修复实现，不能创建豁免；
6. 预期变化必须建立 GitHub issue，说明玩家可见影响和兼容性；
7. 在 `waivers/` 新增一个只针对单个 fixture 的 JSON；
8. 获得政策要求数量的明确批准；
9. 先提交豁免，再更新对应 fixture；
10. CI 同时验证 policy、waiver 和全部 fixture。

## 4. 差异豁免 v1

文件名必须与小写 `id` 完全一致，例如 `waiver-2026-001.json`：

```json
{
  "schemaVersion": 1,
  "id": "waiver-2026-001",
  "status": "approved",
  "fixtureId": "combat.melee.basic-hit",
  "changeKind": "intentional-rule-change",
  "affectedAssertions": [
    "events",
    "finalState.stateHash"
  ],
  "oldNormalizedHash": "0000000000000000000000000000000000000000000000000000000000000000",
  "newNormalizedHash": "1111111111111111111111111111111111111111111111111111111111111111",
  "reason": "Describe the exact rule correction and player-visible effect here.",
  "issue": "https://github.com/UncleFvcker/RoguelikeFansBand-Rewrite/issues/123",
  "approvedBy": ["maintainer-github-login"],
  "approvedAt": "2026-07-15",
  "expiresAt": null
}
```

`changeKind` 只允许：

- `intentional-rule-change`：主动修正规则；
- `legacy-divergence`：明确决定不复刻旧版行为；
- `normalization-change`：规范化语义发生版本内修正。

## 5. 机器验证规则

- policy Schema、contract Schema、normalization Schema 必须匹配代码常量；
- 旧版 commit 必须是固定的 `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`；
- fixture 数量不得低于 20；
- policy 中的目录只能是 policy 文件下的安全相对路径；
- waiver ID 必须是小写字母、数字和连字符，并与文件名一致；
- waiver 必须引用现存 fixture；
- 每个 fixture 最多存在一个活动豁免；
- assertion scope 必须逐项列出，禁止通配符；
- old/new hash 必须是不同的小写 SHA-256；
- 原因至少 20 个字符；
- issue 必须是 GitHub issue URL；
- 批准人必须去重且不能是空值或 `TODO`；
- 日期必须使用 `YYYY-MM-DD`，到期日不能早于批准日。

## 6. Schema 或基准升级

以下变化不能使用普通 waiver 处理：

- 更换旧版基准 commit；
- contract Schema major 变化；
- normalization Schema 变化导致大范围 hash 变化；
- state hash Schema 变化；
- 删除一个已发布 fixture 集合。

这类变化必须新增版本目录和 policy，例如 `contract-v2/`，保留 v1 作为历史回归入口，并提供明确迁移说明。

真实 `.rfbcontent` 激活已经按此规则建立 `contract-v2`；背包权威状态随后建立 `contract-v3`；装备与批量丢弃建立 `contract-v4`；装备属性和稳定拆堆实例建立 `contract-v5`；基础攻击/防御和临时权威伤害公式建立 `contract-v6`；RFB 风格基础近战、受伤与死亡闭环建立 `contract-v7`；行动能量、速度、世界脉冲和怪物追踪建立 `contract-v8`；状态、抗性和毒素 tick 建立 `contract-v9`；流血与内容驱动元素近战建立 `contract-v10`。当前规则边界见 [Contract v10](contract-v10-bleeding-elemental-melee.md)。
