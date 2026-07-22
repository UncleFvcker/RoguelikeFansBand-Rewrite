# Contract v47：深度主题 Vault 与群体遭遇

状态：已实现

## 1. 纵切边界

本纵切为 Stage E 建立第一类可复用、内容驱动的 vault：

- 内容包新增独立 `vaults` 根与 `VaultDefinition` Schema；
- 普通程序化楼层可通过稳定 `themeId` 和 `vaultId` 引用模板；
- 模板声明尺寸、基础 terrain、入口、terrain 覆盖、群体成员位置和主题 loot 位置；
- 群体使用带权 actor 表，并按 `minDepth/maxDepth` 与 actor level 双重过滤；
- vault loot 继续复用 contract-v24 的加权物品/品质/词条生成事务；
- 模板 terrain、群体实体、掉落实例和 RNG 都随当前/离层 `FloorState` 保存并参与既有 state hash 投影。

首个原创模板 `demo.vault.harmonic-sepulcher` 位于回声地牢深度 2：一个隐藏入口的 6×5 封闭墓库，生成 3 个深度合格的哨兵和一份专属缓存。

## 2. 验证与确定性

编译器拒绝：

- 越界、重复或落在阻挡 terrain 上的怪物/掉落位置；
- 无入口、非法尺寸、悬空 terrain/actor/loot 引用；
- 零权重、非法深度区间、玩家 actor 或超过区间上限的 actor；
- 楼层主题与 vault 主题不一致；
- 引用 vault 的楼层没有任何当前深度可选项，或同时把普通 remote-room spawn 填进同一房间。

模板与子项按稳定 ID/位置规范化。每个群体成员固定消费一次加权 encounter 抽取；每个 loot roll 仍固定消费物品、品质、词条三次抽取。协议升级到 1.47，用于拒绝以旧生成规则解释新回放；save 容器仍为 v1，state hash 仍为 Schema v19，因为没有新增权威状态字段，生成结果已经由 terrain、actor、item、RNG 与 content hash 覆盖。

## 3. 兼容与基准

- 内容包：1.40.0；
- content hash：`ae7b19dd780d73091a5b34aed2f67dcbc5650d2e2ed1d7748cc86f48020f8fb0`；
- terrain 35、actor 8、affix 1、item 5、loot table 5、vault 1、world 1；
- `e03cb30ea8e1cd5821c14b54c4a038d30323cfc2cb6e0d6c483cbb006d70916f` 加入已知内置内容迁移表；旧存档已有楼层保持保存下来的原布局，重新开始的探索使用新 vault；
- contract-v47 从 v46 迁移 91 个 exact fixtures，并新增 `92-themed-vault-group.json`；
- active baseline 共 92 个 exact fixtures，0 waiver。

## 4. 与原版 RFB 1.3.0.7 对比

### 相同点

- 原版 `lib/edit/v_info.txt` / `vaults.txt` 把 vault 作为带等级范围、稀有度、地图图案、怪物和物品指令的内容模板；本实现同样把 vault 从生成代码中抽成可验证模板，并让具体格子同时承载 terrain、怪物和 loot 语义。
- 原版 `rooms.c` 在套用模板时先放地形，再在第二阶段放怪物和物品；本实现也先绘制基础 terrain/覆盖，再生成 encounter group 和 loot。
- 原版模板怪物通过 `monster_level`、分配 hook 和 `get_mon_num_aux()` 受深度/主题限制，物品通过 `object_level` 和主题生成；本实现同样让楼层深度过滤 encounter 候选，并让 vault 引用独立主题 loot table。
- 原版允许模板格使用 `PM_ALLOW_GROUP`，普通怪物还可通过 `RF1_FRIENDS` / escort 扩展为群体；本实现已能由模板一次生成多个稳定成员。

### 主动差异

- 原版模板是字符地图和 `L:` 指令，仍包含许多历史硬编码字符；本实现使用严格 JSON、显式 terrain ID、稳定子 ID 和坐标，不让字符含义渗入核心。
- 原版可旋转模板、按层级/稀有度随机选择 lesser/greater vault，并允许大量越深层怪物；当前模板由楼层显式引用，位置与规模固定，候选必须满足声明深度和 actor level。
- 原版朋友/护卫群体由怪物种族旗标和运行时散布生成，并带 pack AI；当前群体是模板声明的固定成员位置，只有生成集合，还没有队伍 AI 或生态上限。
- 原版 vault 掉落可使用 best-of、多种 `object_level` 增幅、神器与 `ORIGIN_VAULT`；当前继续使用统一 loot table、质量和词条事务，没有专门提高越级或神器概率。

### 暂未实现

- 按深度、稀有度和地牢主题加权选择多个 vault，旋转/镜像和自由房间落位；
- 普通房间也可复用的独立 encounter/loot/theme 表，以及巢穴、兽栏、pit 和 formation；
- `friends`、escort、召唤、繁殖、pack AI、种群上限和 unique 过滤；
- vault 专属陷阱密度、越级强敌/掉落、神器、来源标签和探索奖励；
- 更大模板、连通性证明、失败回退和多 vault 楼层预算。

这些缺口已同步写入 [待实现内容清单](pending-implementation.md)。
