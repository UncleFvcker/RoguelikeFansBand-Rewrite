<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Original content packs

本目录只包含为 RoguelikeFansBand Rewrite 单独创作或按已记录兼容行为独立表达的游戏数据，按 CC BY-SA 4.0 授权。

旧 RoguelikeFansBand、FrogComposband 和 Angband 的地图字节、描述文本、算法、成套数值表与素材不会复制到这里。`rfb-demo-original` 是用于验证稳定 ID、内容编译、地图与 tileset 接口的独立实现包；Phase 17 的 Warrens 批次按固定来源记录的层数和 gameplay 角色重新表达，差异见 [`contract-v150-warrens-journey.md`](../design/contract-v150-warrens-journey.md)。按用户明确选择加入的 Warrior 小切片只固定该职业、Standard 身体和四件出生物品所需的少量字段，描述仍为重新撰写，未开放批量迁移，边界见 [`contract-v151-warrior-and-dungeon-status.md`](../design/contract-v151-warrior-and-dungeon-status.md)。

当前 `rfb-demo-original` 为内容包 `1.142.0`，content hash 为
`dd7a374770e13e923ac7c2be0648e3fea2793bcec5b78c81adf90f3d30783c36`。
当前包共 49 terrain、33 actors、96 items、68 abilities、5 ability books、4 affixes、3 resources、13 skill sets、5 races、7 builds、7 encounter tables、10 loot tables、6 vaults 和 2 worlds。生产 New Game 使用 Warrior 进入只包含 Warrens 战役的独立世界；Original Lab 与旧 demo builds 继续承载既有系统回归。完整内容锁定见 [`content.lock.json`](rfb-demo-original/content.lock.json)。
