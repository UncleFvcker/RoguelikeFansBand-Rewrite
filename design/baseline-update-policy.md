# Contract 基准更新政策

状态：active-only Policy v2 已由 `rfb-contract` 和 CI 强制验证

## 1. 目的

contract fixture 是规则兼容边界，不能把测试失败简单处理为“刷新预期结果”。政策用于保证每次规则变化只修改真正受影响的场景，同时保留可审查的失败原因。

当前逻辑基线是 `contract-v255`，机器可读政策固定在：

```text
tests/fixtures/active/baseline-policy.json
```

`contract-v255` 为既有怪物近战物理伤害增加窄 `vampiric` 标记，以实际结算伤害
治疗攻击者；非生命玩家仍承受伤害，但不会提供治疗。新增 3 条 Orc Cave 记录。
上一批 10 条地点范围不属于 Orc Cave 的记录继续保留为全局内容，仅移除
`orc-cave` 归属；严格选择现有 179 条 Orc Cave 记录。冷血红外规则保持不变；
协议保持 1.169，State Hash Schema 保持 v85，save 容器保持 v1。

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

`refresh-category` 会先为选中分类计算全部断言；任一场景计算失败时不会写入该批文件。普通 `cargo test -p rfb-contract` 仍快速检查全部 fixture 的 JSON、schema、分类和 ID 唯一性，但不会运行 22 条完整回放。

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

禁止批量 refresh 未受影响场景，也禁止为了“让测试通过”降低最低 fixture 数量。退役内容或删除已由核心单元测试覆盖的重复矩阵时，可以随新 baseline 一并降低数量，但保留集必须通过 policy 与 `verify-all`。仅改变默认字段是否落盘的全量重写属于表示迁移，必须在不调用 `observe` 的情况下完成，并证明重写前后的反序列化对象完全相等。

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

`contract-v238` 退役 Original Lab / Echo / Resonance fixture，并将 active 集收敛为 21 个跨协议、存档和关键状态边界的 exact fixture。怪物、装置和地下城布局矩阵由对应核心单元测试负责；active 集零 waiver。

`contract-v239` 将 fixture schema 升至 v4：能力断言只保存 ID、熟练度、施法统计和剩余冷却，build 只在成长分类保存 ID，任务只在任务分类保存运行状态，地图变更格只由移动、地牢和城镇分类收集。完整 `AbilityDto` 由单独的协议投影测试保护。

`contract-v243` 完成主动变异 M5-C/D。Sterility 把当前楼层的怪物繁殖压制纳入
权威存档与 State Hash Schema v81；21 个 fixture 只刷新受该公共哈希结构影响的
`stateHash` 与 `saveRoundTripStateHash`，具体九项能力分支由聚焦核心测试覆盖。

`contract-v244` 完成周期变异 M6-A。`minorSlow` 与 Produce Mana 的待选方向进入
权威存档和 State Hash Schema v82；协议升至 1.165。21 个 fixture 刷新公共状态
投影与哈希，十项周期分支、RNG 顺序和方向恢复由 Core 与 Web 聚焦测试覆盖。

`contract-v245` 完成周期变异 M6-B。通用现实改变倒计时进入权威存档和 State
Hash Schema v83，协议升至 1.166。21 个 fixture 刷新公共状态投影与哈希；传送、
放逐、程序地下城重生成和武器掉落由 Core 聚焦测试覆盖。

`contract-v246` 完成周期变异 M6-C。Flatulence、Raw Chaos 与 Eat Light 复用范围
伤害，三类吸引复用分类召唤和角色群体生成；照明模块增加来源无关的区域熄灭事务。
本批没有协议、存档或 State Hash 结构变化，现有 21 个 fixture 无需刷新，具体分支
由 Core 聚焦测试覆盖。

`contract-v247` 完成周期变异 M6-D。Normality、Polymorph Wounds、Wasting、
Random Telepathy、Nausea、Warning 与 Wraithform 复用变异移除、属性、状态、饥饿
和实体等级事务。本批没有协议、存档或 State Hash 结构变化，现有 21 个 fixture
无需刷新，七项周期分支和 RNG 顺序由 Core 聚焦测试覆盖。

`contract-v248` 完成变异 M7。变形药水复用唯一随机 gain/lose 事务，并保持锁定
保护、互斥移除、原版循环与精确 RNG 顺序。药水加入两座城镇的 Black Market，
固定库存改变新游戏公共初始化与物品实例分配，因此 21 个 fixture 全量刷新；算法
分支、零候选和物品消耗由 Core 聚焦测试覆盖。协议、存档和 State Hash 结构不变。

`contract-v249` 完成 M4F-C 的 Good/Bad Luck、Easy Tiring 与 Impotence。幸运统一
影响随机变异权重、物品质量/生成深度和永久属性提升；易疲劳复用 `minorSlow` 并
新增持久的恢复能量；魔法无能在共享设备检定入口按 staff/rod 与特殊效果修正技能。
`minorSlowEnergy` 使协议升至 1.167、State Hash Schema 升至 v84，21 个 fixture
全量刷新；具体概率、疲劳恢复和设备类别分支由 Core 聚焦测试覆盖。

`contract-v250` 完成随机候选 Chaos Gift。所有新角色出生时确定并持久化一位 RFB
权威混沌神明；拥有该变异的角色首次达到新最高等级时按原版奖励表结算。神明身份
增加公共初始化 RNG 与存档字段，因此协议升至 1.168、State Hash Schema 升至 v85，
21 个 fixture 全量刷新。奖励选择、最高等级门槛和混沌武器等级表由 Core 聚焦测试
覆盖；104 个随机候选至此全部 active。
