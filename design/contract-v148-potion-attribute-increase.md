# Contract v148：P97 属性永久增长药水

日期：2026-07-30

Contract v148 接入六种单属性增长药水与 Augmentation。协议保持 `1.123`，demo 内容包升至 `1.139.0`，save 容器保持 v1，state hash Schema 保持 `55`。active baseline 包含 452 条 exact fixtures、零 waiver；内置内容 hash 为 `a8eb3c1a5b74f683bd5a71728da916f67972088769e3155cdc0b89c88b4e874c`。

## 1. 内容效果

物品效果新增 `increase-attribute { attribute }` 与无参数 `augment-attributes`。单属性效果覆盖 Strength、Intelligence、Wisdom、Dexterity、Constitution 和 Charisma；Augmentation 固定按 STR、INT、WIS、DEX、CON、CHA 顺序处理六项属性，不开放任意属性列表或通用成长事务。

demo 新增原创 Might Tonic 与 Sixfold Tonic。legacy importer 映射 tval 75/sval 48–53 和 55，使 `consumable-effect` 从 53 降至 46。

## 2. 增长与知识语义

每项增长先把当前自然属性恢复到历史最大自然属性，再按原版三段公式增长历史最大值：18 以下使用一次百分比检定，18 以上且未接近上限时按距上限比例增长，最后两档固定增加 1。增长继续使用胜利前 `18/220`、胜利后 `18/820` 上限；已达上限时不抽效果 RNG，也不阻止 Augmentation 处理后续属性。

恢复或历史最大值增长任一实际发生时，来源物品变为 Aware；完全无变化时只保留 Tried。药水增长不消费等级提升积累的 `pendingAttributeIncreases`。六项处理完成后只刷新一次 HP、职业资源上限和派生属性。

## 3. 事件与 Fixture

属性事件增加 `increased` 结果。增长前存在损伤且已达历史上限时，事件明确投影为 `restored`；真正提高历史最大值时投影为 `increased`。Augmentation 先汇总整瓶药水是否可察觉，再投影全部事件，避免同一事务中混用未知和已知物品名。

fixture 452 固定四次使用：已封顶的 Might 只记 Tried；Frailty 损伤 Strength；第二瓶 Might 只恢复 Strength 并变为 Aware；Sixfold 跳过封顶 Strength，以 9 次效果 RNG 增长其余五项。最终自然/历史最大属性为 STR 238、INT 146、WIS 148、DEX 145、CON 153、CHA 156。

## 4. 导入与版本

固定原版源码导入保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors、68 races 和 54 classes。真实导入内容 hash 为 `2a5a78a6c8518385e45babebcc2670edd9ddb653a1eca8da2c78635c497e1138`，编译文件 SHA-256 为 `1182790D1FB24B422ED2B8FE42E9BD4EB2ACCF1F2D64C1F08105CFF2EBE474E8`。

内容 Schema、中英 Fluent key 和 Web outcome 同步更新。内容 hash 改变，因此 active fixture 的 hash 字段刷新；hash 输入结构未改变，协议与 state hash Schema 均不升级。
