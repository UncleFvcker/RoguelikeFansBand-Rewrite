# Contract v274：圣骑士职业能力

状态：已实现。

## 权威来源

- `D:/codex/Frogcomposband` 的 `master` ref，核对提交
  `efd63661302866038f58d8cd2553b23e6af3bf9d`。
- `src/paladin.c`：邪恶圣骑士 30 级 WIS 职业能力、Mana 消耗 30、失败参数 70，以及
  40 级恐惧抗性。
- `src/spells_h.c`：名称“地狱长枪”、说明“发射一道纯粹的地狱火射线。”和伤害
  `spell_power(level × 3 + to_d_spell)`。

## 职业能力

- 正式占用 `demo.ability.paladin-hell-lance` 和
  `demo.ability-program.paladin-hell-lance`；没有新增 item ID。
- 30 级开放，使用 WIS、`demo.resource.mana` 30 点、基础失败参数 70，并继承 Paladin
  最低失败率 5。
- 使用通用方向目标和地狱火 beam。基础能力以固定 `1d1` 加等级缩放表达恰好
  `level × 3`，随后应用 casting profile 的 `spellDamageBonus`。

## 等级抗性

- `ClassDefinition` 新增可选 `levelResistances`，每项声明 `minimumLevel` 与一个非空抗性
  表；编译器排序并拒绝重复等级、越界等级和空表。
- 玩家有效抗性在 actor、状态和 race 之后合并已达到门槛的 class 抗性。Paladin 在
  40 级获得 `fear: resistant`，核心没有圣骑士 ID 特判。
- 抗性完全由当前 class 与 level 派生，不新增持久字段、协议字段或状态哈希输入。

## 投影与验收

- 既有 Ability DTO/UI 自动显示 class power 的名称、说明、等级、消耗、失败率、beam
  细节和可施放状态；29 级仍可查看但不可施放，30 级开放。
- 定向测试实测 30 级地狱长枪基础伤害 90、40 级恐惧抗性，并覆盖新存档往返；出生、
  神授学习和 32 条死亡祈祷参数测试继续通过。
- 内容包为 1.274.0，content hash 为
  `e94926512734080f4743341e0eff07e3c96f371fe8cdac674089654b28fa2010`。Protocol 1.177、
  State Hash Schema v88 与 save v1 不变；active baseline 升至 contract-v274。

## 仍未导入

- 摧毁高级异教书获得经验依赖未来正式导入的生命/圣战高级书。本批不创建占位书，
  不占用 items 方向 ID。
- 原版逐武器熟练度仍属于共享内容模型缺口，见
  [`legacy-class-import-v1.md`](legacy-class-import-v1.md) 第 3 节。
