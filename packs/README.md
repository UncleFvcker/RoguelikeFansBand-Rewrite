<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Original content packs

本目录只包含为 RoguelikeFansBand Rewrite 单独创作的原创游戏数据，按 CC BY-SA 4.0 授权。

旧 RoguelikeFansBand、FrogComposband 和 Angband 的数据、文本、专名与素材不会复制到这里。`rfb-demo-original` 是用于验证稳定 ID、内容编译、地图与 tileset 接口的最小原创包。

当前 `rfb-demo-original` 为内容包 `1.102.0`，content hash 为
`12c9160aec3bf8ebc6b7c92a785ad1ed8ad2dd23af674bd4bc6c445d2762d2e7`。
它在既有角色成长、构筑、技能、地牢生成、玩家/怪物施法、召唤物命令、装备旗标和动态 affix 纵切之上，包含 Death 四本原创能力书的 32 个对应能力。P58 的 Resonance Mender 验证固定充能设备；P59 新增 Resonance Wand/Staff/Rod，以动态实例 profile 覆盖深度加权 bolt、自疗、持久陷阱侦测、目标前置拒绝、知识门控及存档回读；P60 再加入 rod 与 wand/staff 的差异化自然恢复，以及 Artificer 的 Resonance 资源/设备来源主动充能；P61 新增 Clarity Draught 与 Perfect Focus Elixir，验证有序状态清除和资源恢复。当前包共 47 terrain、28 actors、25 items、68 abilities、5 ability books、4 affixes、3 resources、13 skill sets、4 races；现有 dungeon/Vault/encounter/loot/theme 内容继续保留。完整内容锁定见 [`content.lock.json`](rfb-demo-original/content.lock.json)。
