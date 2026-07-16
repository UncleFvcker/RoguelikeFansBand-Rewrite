# Contract v6 基础战斗属性迁移

状态：active contract baseline

## 1. 迁移范围

`contract-v6` 在 v5 的装备、稳定物品实例 ID 和部分数量丢弃基础上，加入 Rust 权威基础战斗属性：

- `ActorDefinition` 新增基础 `attack` 和 `defense`；
- `StatModifiers`/`StatModifiersDto` 新增可正可负的攻击、防御修正；
- `PlayerDto` 同时输出基础攻击/防御、装备修正和最终攻击/防御；
- `EntityDto` 输出内容定义派生的攻击与防御；
- 背包与装备 UI 显示攻击、防御和最大生命词条；
- HTML 状态层显示玩家最终攻击、防御及装备来源；
- 近战伤害由 Rust 核心根据攻击、防御和权威 RNG 计算。

存档容器和 `SavePayloadV1.schemaVersion` 不变。新增 DTO 字段使用默认值兼容旧 MessagePack，载入后从当前内容定义和权威装备列表重新派生，不信任存档中的重复战斗数值。

## 2. 当前伤害公式

第一版只处理玩家撞击怪物的近战伤害：

```text
damage = max(1, attacker.attack + rng(0..1) - defender.defense)
```

原创探索者基础攻击为 2，发光微粒防御为 1，因此未装备时仍产生原有的 1–2 点伤害序列，不改变既有战斗 fixture 的事件顺序。回声护符提供：

- 攻击 +1；
- 防御 +1；
- 最大生命 +4。

装备后玩家攻击从 2 变为 3，对同一微粒的固定种子前两次伤害由 1、1 变为 2、2。玩家防御已经进入权威属性模型，但怪物主动攻击尚未实现，因此暂不参与受击结算。

## 3. 内容与兼容性

原创内容包升级为 `1.3.0`，content hash 为：

`36bdba260173b9ba7477e85b886c134affed0369aa4f7a485e59e4408e618ebd`

核心显式兼容内置内容包 1.0.0、1.1.0 和 1.2.0 的已知 hash。旧存档缺失攻击/防御字段时使用零作为“未保存”标记，并从 1.3.0 内容定义恢复基础属性；已装备的回声护符自动获得当前三个修正。未知或模组 hash 仍拒绝载入。

## 4. State hash 与 Contract

玩家、实体和装备 DTO 增加战斗属性，内容 hash 与战斗结算也发生变化，因此：

- 协议升级到 `1.6`；
- `stateHashSchemaVersion` 从 5 升级到 6；
- active baseline 迁移到 `contract-v6`；
- v1–v5 继续作为历史迁移记录保留。

v6 包含 29 个 exact fixtures。新增 `combat.melee.equipped-attack-modifier`，覆盖拾取、装备、路径移动、2/2 点伤害、击杀、事件顺序和存档回环。baseline policy 为 `rfb-contract-baseline-v6`，无 waiver。

## 5. 验证命令

```powershell
cargo test -p rfb-content -p rfb-protocol -p rfb-core -p rfb-save -p rfb-replay
cargo test -p rfb-contract
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v6/baseline-policy.json
cd web
npm run typecheck
npm test
npm run e2e
```
