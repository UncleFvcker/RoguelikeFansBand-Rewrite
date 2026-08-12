# Contract v280：通用法术强度

状态：已实现。

## 权威规则

- RFB `master:src/do-spell.c` 的共享公式为
  `max(0, value + value * spellPowerBonus / 13)`，整数除法向零截断。
- 装备与 affix 的 `SPELL_POWER`、`DEC_SPELL_POWER` 按各自 pval 增减法术强度；
  当前有效总值由已装备物品、其 affix/随机 affix，以及活动状态的 modifier 汇总。
- 能力内容只列出源代码实际调用 `spell_power` 的字段，不把缩放隐式施加到整个能力。

## 能力与投影

- 32 个死亡法术全部完成核对；24 个含源 `spell_power` 调用的法术显式标记最终伤害、
  伤害骰面/加值、power、半径、持续时间基数/骰面、鉴定阈值或随机选择值。
- Invoke Spirits 只缩放外层选择值，23 个分支自身不二次缩放；末分支上界扩为
  `u16::MAX`，确保正法术强度仍能稳定命中最终分支。
- 最终伤害和随机选择保留“先掷骰、后套共享公式”的源顺序；协议投影携带对应的
  法术强度加值，其余定值字段在投影和实际结算中复用同一物化路径。
- 未引入算术表达式 DSL、递归组合或兼容旧开发存档的分支。

## 导入与验证

- legacy importer 将物品、ego 与神器的 `SPELL_POWER`、`DEC_SPELL_POWER` 映射为
  `spellPowerBonus`，并为死亡领域生成相同的显式字段标记。
- 聚焦测试覆盖负值归零、零值、正值、整数截断，以及 Blood Rite 风格的状态 +7；
  同时覆盖装备、固定 affix、随机 affix、投影与实际伤害结算。

## 版本与基准

- 集成内容包 1.282.0，content hash 以当前 `content.lock.json` 为准。
- Protocol 1.182、State Hash Schema v91，save v1 不变。
- `StatModifiersDto` 新增 `spellPowerBonus`，改变全部 active fixture 的公共投影和状态哈希；
  因此统一刷新 active exact fixture，行为基线提升为 contract-v280，零 waiver。
