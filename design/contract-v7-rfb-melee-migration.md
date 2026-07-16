# Contract v7：RFB 风格基础近战闭环

状态：active contract baseline

## 1. 参考边界

本阶段只读参考旧 RFB 1.3.0.7 的规则实现，不复制旧代码、文本、数据或素材。主要行为依据是玩家普通近战的 `test_hit_norm()`、怪物普通近战的 `_check_hit()` / `make_attack_normal()`，以及普通 `HURT` 攻击的 AC 减伤。

第一轮复刻影响基础手感且能由当前垂直切片验证的部分：

- 5% 自动命中、5% 自动失误；
- 攻击能力对抗目标 AC；
- 怪物等级参与近战命中能力；
- 内容定义的整数伤害骰；
- 普通物理伤害按 AC 线性减免，AC 180 时达到 60% 上限；
- 玩家回合结束后，邻接怪物按稳定实例 ID 顺序反击；
- 玩家受伤、生命降到 0 以下死亡、死亡事件、存档/回放和死亡后命令拒绝。

暂缓：不可见目标惩罚、多段 blow、武器 `to_d`、暴击、斩杀、品牌/克制、偷窃/吸取、异常状态、反击光环、无敌与职业特殊减伤。这些能力必须以后通过独立规则组件和新 contract 扩展，不能堆进基础命中函数。

## 2. 当前确定性公式

当前原创内容仍使用简短的 `attack` / `defense` 评级。核心先映射为 RFB 风格内部尺度：

```text
player_skill = effective_attack * 20
monster_skill = monster.attack * 20 + max(monster.level, 4) * 3
armor_class = effective_defense * 10
```

命中检查使用 Rust 权威 RNG：

```text
percentile = rng(0..99)
if percentile < 10:
    hit = percentile < 5
else:
    hit = rng(0..skill-1) >= floor(armor_class * 3 / 4)
```

玩家攻击能力小于等于 0 时不会触发自动命中。当前没有隐身状态，因此尚未应用旧版“不可见目标命中能力减半”。

角色内容新增 `damageDice` 和 `damageSides`。命中后逐骰抽取并求和；玩家攻击不会再用临时的“攻击减防御”伤害公式。怪物普通物理伤害随后计算：

```text
reduction = 60 * clamp(armor_class, 0, 180) / 180
damage = raw_damage * (100 - reduction) / 100
```

整数截断可能让很小的伤害降为 0，这与参考规则的整数计算一致。玩家生命值小于 0 时死亡；生命值恰好为 0 时仍未死亡。死亡后的命令返回 `PlayerDead`，不会推进回合、revision、RNG 或回放。

## 3. 协议、内容与兼容性

- 协议升级到 `1.7`；
- state hash Schema 升级到 `7`；
- active baseline 升级到 `contract-v7`，共 32 个 exact fixtures；
- 原创内容包升级到 `1.4.0`；
- content hash：`d0537220f093719e623b51bf589dd0a3d8a67ccdc534a1502adcebe094120e9b`；
- 核心继续显式接受内置内容包 1.0.0–1.3.0 的已知 hash，并从当前内容重新派生战斗字段；
- `PlayerDto` / `EntityDto` 新增内部近战能力、AC 与伤害骰，玩家 DTO 额外输出 `isDead`；旧存档缺失字段时使用默认值并由当前内容派生。

新增 fixtures 固定怪物反击与受伤存档回环、玩家失误及反击事件顺序、死亡事件、负生命值保存恢复和死亡后的命令拒绝。

## 4. 验证

```powershell
cargo test -p rfb-content -p rfb-protocol -p rfb-core -p rfb-save -p rfb-replay
cargo test -p rfb-contract
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v7/baseline-policy.json
cd web
npm run typecheck
npm test
```
