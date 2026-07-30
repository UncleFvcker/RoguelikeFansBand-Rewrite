# Contract v147：P96 属性事务修正与 Sustain

日期：2026-07-30

Contract v147 修复 contract-v146 的属性事务问题，并让六种属性 sustain 重新具备权威规则消费者。协议升至 `1.123`，demo 内容包升至 `1.138.0`，save 容器保持 v1，state hash Schema 保持 `55`。active baseline 包含 451 条 exact fixtures、零 waiver；内置内容 hash 为 `2b1bf5beabe42513d3ad70e0d536274a773babf391c085f3af4ca7a720a2e003`。

## 1. 资源比例

属性变化前同时保存每个资源池的 current 与 maximum。刷新新上限后，current 只按 `oldCurrent * newMaximum / oldMaximum` 计算一次，不再使用已经被新上限 clamp 的 current。HP 的既有比例刷新保持不变。

## 2. 属性 Sustain

`EquipmentPassive` 与协议 DTO 恢复六种 `sustain-*`。属性损伤事务先检查已装备 passive：命中对应 sustain 时不调用属性损伤、不抽效果 RNG，当前值与历史最大值都不变；来源消耗品仍被察觉并变为 Aware，同时发布 `item.use-attribute-sustained`。Warding Band 作为原创 exact contract 载体提供 `sustain-strength`。

legacy importer 将 `SUST_STR/INT/WIS/DEX/CON/CHR` 映射到对应 passive。固定源码导入后，可装备 ego 与 artifact 的六类 sustain gap 清零；唯一剩余 `SUST_CHR` 属于 slotless artifact，继续保留在 gap 报告。真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；内容 hash 为 `21fb38c839a993bcb5b2b6562a7ff46ce537255052fa4ef41bebc4db00a245c3`，编译文件 SHA-256 为 `51EE565B95C6A3CCB9A8BBE1FCC7F7A4D84DFB9CD005C9F0F5554ED7B83A0074`。

## 3. Fixture Schema

contract fixture schema 升至 2，同时继续读取 schema 1。只有 schema 1 且六项 `maximumNatural` 全为零时，验证器才从对应 `natural` 显式迁移旧投影；部分填充直接返回错误，schema 2 不执行该迁移。历史 449 条 fixture 保持 schema 1，fixture 450 与后续场景使用 schema 2。

fixture 451 装备 Warding Band 后使用 Frailty Tonic，固定 Strength `38 -> 38`、药水消费与 Aware、sustained 事件、零属性损伤 RNG和 save round-trip。fixture 450 继续覆盖正常损伤、恢复和无变化路径。

## 4. Web 与版本

Web 属性提升按钮使用 `maximumNatural >= attributeCap` 判断历史上限，不再以被损伤后的当前桶误启用按钮。协议枚举、TypeScript bindings、内容 Schema 和中英 Fluent key 同步更新。内容 hash 改变，因此 active fixture 的 hash 字段刷新；hash 输入结构未改变，state hash Schema 不升级。
