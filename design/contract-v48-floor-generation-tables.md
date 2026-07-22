# Contract v48：楼层生成表、加权 Vault 与巢穴

## 目标

本切片把程序化楼层的普通房间怪物、楼层掉落和主题选择从楼层内联配置提升为可复用内容表，并让同一主题可以按深度和权重选择多个 Vault。它同时建立第一类巢穴：一次选择怪物种类，在指定房间生成多个同类实例。

## 内容模型

- 新增独立 `EncounterTableDefinition` 根。表声明稳定 ID、每层抽取次数，以及带 `weight/minDepth/maxDepth` 的怪物候选。
- 新增独立 `ThemeTableDefinition` 根。主题候选声明 `themeId`、楼层 terrain、深度与权重，并可包含多个带独立深度范围和权重的 Vault 候选。
- `ProceduralFloorDefinition` 新增 `encounterTableId`、`lootTableId`、`themeTableId` 和可选 `nest`。`nest` 固定房间 ID 与生成数量。
- 旧 `themeId/vaultId/actorSpawns/lootSpawns` 继续作为兼容输入，但同一楼层不能把新表引用与对应旧字段混用。
- 编译器验证所有引用、正权重、深度区间、怪物角色/等级、可行走 terrain、主题一致性、Vault 房间适配和当前楼层至少一个合法候选。

原创 demo 包升级到 1.41.0，包含一个 encounter 表、一个 theme 表、五个 loot 表和两个同主题 Vault。三层回声地牢都从楼层表选择普通遭遇与掉落；深度 1 生成一个三成员同类巢穴，深度 2 在 `harmonic-sepulcher` 与 `resonant-gallery` 之间执行 3:1 加权选择，深度 3 因无合法 Vault 候选而稳定回退为普通房间。

## 确定性生成顺序

首次生成楼层时按以下顺序执行：

1. 过滤 theme 表的深度候选；只有一个合法主题时不消费 RNG，多个候选时执行一次加权抽取。
2. 过滤所选主题中深度合格且能放入目标房间的 Vault；无候选时不消费 Vault 抽取并继续普通生成，一个候选直接选择，多个候选执行一次加权抽取。
3. 使用主题声明的 floor terrain 绘制房间、走廊和可选 Vault terrain。
4. encounter 表按稳定 entry 顺序过滤深度，每次 roll 分别执行怪物种类加权抽取和既有稳定位置选择；普通遭遇避开已经放置 Vault 的房间。
5. 巢穴只抽取一次深度合格怪物种类，再按稳定实例序号选择多个位置，所有成员共享该种类。
6. 依次生成 Vault encounter group、守护者、怪物携带物、楼层 loot table 和 Vault loot；各自继续使用既有确定性生成事务。

所有候选在编译阶段规范化排序；权重计算只使用整数。相同 seed、内容 hash 和命令序列必须选择相同主题、Vault、怪物、位置与掉落。

## 兼容性

- 协议升级到 1.48，用于拒绝以 1.47 生成规则解释新的首次楼层状态与回放。
- save 容器保持 v1，state hash 保持 Schema v19；生成后的 terrain、actor、item、实例分配器、RNG 和 content hash 已在现有投影中覆盖。
- v47 内置内容 hash 被列为可迁移来源。旧存档中已经生成的楼层保持原 terrain、actor、item 与 RNG，不补套生成表、Vault 或巢穴；尚未生成的楼层按 v48 内容生成。
- contract-v48 从 v47 迁移 92 个 exact fixtures，并新增 4 个 fixture，覆盖深度过滤与同类巢穴、两个加权 Vault 候选、无候选回退和存档回环。active baseline 共 96 个 exact fixtures，0 waiver。

## 固定版本与规模

- 协议：1.48；
- 内容包：1.41.0；
- content hash：`9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad`；
- 内容：terrain 35、actor 8、affix 1、item 5、encounter table 1、loot table 5、theme table 1、vault 2、world 1；
- save：v1；state hash：Schema v19；
- active baseline：96 exact fixtures，0 waiver。

## 明确遗留

- 十层以上地牢、整层预算、多区域主题、多个 Vault 同层放置和生成压力场景；
- Vault 旋转、镜像、自由房间落位、多入口、大模板连通性证明和失败重试策略；
- pit、formation、friends、escort、召唤、繁殖、pack AI、种群上限与 unique 过滤；
- 巢穴的专属表、形状、领袖、主题掉落、唤醒/生态规则；
- Vault 越级强敌、专属陷阱、神器、来源标签和探索奖励；
- 分支、shaft、随机楼梯与显式到达点。
