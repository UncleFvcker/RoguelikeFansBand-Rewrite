# Contract v22：实例词条与知识投影

状态：协议 1.22 / contract-v22 active baseline

## 已建立边界

- 内容格式新增独立 `affixes` 根与 `AffixDefinition`。世界物品实例用稳定 `affixIds` 引用词条；首个固定词条 `demo.affix.harmonic-edge` 为回声护符提供攻击 +1。
- 词条是真实的实例级权威状态，进入 save v1 的可选 `affixIds`、显式实例知识记录与 state hash Schema v11。旧存档缺少这些字段时仍按空值载入。
- 未发现词条不会进入玩家可见的 `modifiers` 或 `knownProperties`。核心始终按真实词条计算规则；当前首次装备是可观察发现触发点，发现与装备在同一事务完成，因此派生数值不会先于知识投影泄漏。
- `item.property-discovered` 事件只携带已公开的物品种类和词条名称 key。HTML 只渲染核心提供的已知属性，不读取实例真值。
- 存档载入拒绝未知、重复、越权的词条引用和知识记录；带词条的已装备实例必须已知其全部词条。
- 原创内容包升至 1.17.0，协议升至 1.22。contract-v22 从 v21 迁移 58 个 exact fixtures，并新增实例词条发现闭环，共 59 个。

## 下一步

伪鉴定与完整识别已由 [contract-v23](contract-v23-item-appraisal.md) 建立：鉴别只公开实例质量，装备公开完整词条。下一步接入掉落表和程序化生成；诅咒知识与其他鉴定来源继续复用该状态机。
