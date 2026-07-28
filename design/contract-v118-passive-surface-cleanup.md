# Contract v118：收缩无消费者的装备 passive 表面

日期：2026-07-28

Contract v118 清理 contract-v103 提前公开、但始终没有权威规则消费者的装备 passive。协议为 `1.118`，demo 内容包为 `1.109.0`，save 容器保持 v1，state hash Schema 保持 `52`，active baseline 保持 420 条 exact fixtures、零 waiver。内置内容 hash 为 `99398a53687b4cf106939ddebcb08865f4a24ee147795e9de2ae8e08036aaf00`。

## 1. 契约收缩

`EquipmentPassive` 与 `EquipmentPassiveDto` 只保留已经进入权威规则的两项：

- `regeneration`：装备后按既有间隔恢复生命；
- `vampiric`：装备武器的有效近战伤害按既有规则吸血。

以下 13 项从内容 Schema、协议 DTO、TypeScript bindings、Web 本地化白名单和 demo 内容移除：`see-invisible`、`telepathy`、`levitation`、`hold-life`、六维 `sustain-*`、`blessed`、`easy-spell`、`device-power`。它们此前可以被声明、保存和显示，但从未改变权威状态或结算。

demo 的 Vampiric affix 只保留 `vampiric`；Adaptive Echo 的深层候选保留设备与豁免技能加值，不再附带 `telepathy`。没有用其他能力替代被删除值。

## 2. 存档边界

历史 rolled-affix 存档可能保存这些 no-op 字符串。兼容只位于 `RolledAffixSaveDto` 的反序列化边界：

- `regeneration` 与 `vampiric` 正常读取；
- 13 个已知历史 no-op 值被丢弃；
- 任何其他未知值继续使解码失败；
- 不重掷 affix、不替换能力、不推进 RNG。

静态 affix 仍由当前内容 hash 迁移到新定义。旧装备不会在未来实现某个原版旗标时自动获得此前不存在的能力；若后续纵切实现真实消费者，应同时重新引入内容枚举、导入映射和 exact contract。

## 3. 真实导入

legacy importer 只继续消费 `REGEN` 与 `BRAND_VAMP`。其余 13 类旗标重新进入 item、ego 或 artifact 的 `unmapped*Flags` 报告；Crown Telepathy 与 Light Scrying 的 telepathy-only roll recipe 被移除。

固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的验证结果：

- item 545/545、ego 122/160、artifact 392/392；
- 编译包包含 122 个 affix；
- 源码校验、编译和二进制回读 hash 均为 `e3408cabe6ca812c8dc3b79f82fadd0322fa18b7f2d8cef119a13b22458f147a`。

这些未映射计数保留未来实现入口，不把尚未支持的规则伪装成已导入能力。

## 4. Fixture 影响

内容 hash 属于 state hash 输入，因此 420 条 active fixture 的 `stateHash` 或 `saveRoundTripStateHash` 都按新内置包刷新。场景 349 另外删除装备 DTO 和已知属性中的两处 `hold-life`；其 `vampiric` 行为与存档回读保持不变。没有新增 fixture、降低最低数量或增加 waiver。
