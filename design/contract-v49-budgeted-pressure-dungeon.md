# Contract v49：预算化十层压力地牢

## 目标

本切片把 v48 的楼层表扩展为可按楼层规模控制的生成预算，并建立一条独立十层压力地牢验证深度增长、主题分段、楼层存储和最终守护者回环。原有三层回声地牢保持独立，用于继续覆盖 Vault、巢穴和旧规则兼容。

## 内容模型与预算语义

`ProceduralFloorDefinition` 新增可选 `generationBudget`：

- `actorSlots`：本层表驱动 actor 的总上限，计入巢穴成员、所选 Vault encounter group 和当前仍应生成的守护者；生成器先为这些固定成员保留位置，再以 encounter table 的 `rolls` 为最大值填充普通遭遇。
- `lootPlacements`：本层生成位置的总上限，计入所选 Vault 的 loot spawn；剩余位置重复调用楼层 `lootTableId`。每次调用仍执行 loot table 自己声明的 rolls。

预算当前只允许普通 dungeon 楼层启用，并且必须同时引用 encounter 与 loot 表。编译器要求 `actorSlots` 为 1–16、`lootPlacements` 为 1–8，并保证每条深度合格 Vault 路径在预留群体/loot 后仍至少能生成一个普通遭遇和一个普通房间掉落。预算是上限：若 encounter table 的 rolls 更小，生成器不会为了填满预算重复超过表上限。

怪物出生携带物不计入 `lootPlacements`；它属于 actor 自身的既有携带物事务。loot table 的内部 rolls 也不按物品实例数扣预算，预算单位明确是生成位置。

## 十层压力场景

demo 包新增 `demo.dungeon.resonance-descent`：

- 独立地表入口和从深度 1 到 10 的线性楼层链；
- 独立 encounter/theme 表，不影响原有回声地牢的 Vault 候选；
- 深度 1–3 使用浅层石地主题，深度 4–10 切换为 `demo.terrain.resonant-floor`；
- actorSlots 从 2 逐层增长到 10，lootPlacements 从 1 增长到 3；
- 深度 10 保留一个守护者槽位，因此生成 9 个普通遭遇和 1 个守护者；
- 每层离开后进入既有 `storedFloors`，最终层支持 save v1 往返和精确 state hash。

## 确定性顺序

预算计算本身不消费 RNG。主题与 Vault 仍遵循 v48 的过滤/权重顺序；确定固定成员数量后，普通遭遇按稳定 ordinal 依次消费怪物权重抽取和房间位置抽取。楼层 loot 按 placement ordinal 选择位置，再执行既有 loot table 物品、质量、词条抽取。所有实例 ID 使用稳定的 `.encounter.N` 与持久化 `generated.item.N` 分配器。

相同 seed、内容 hash 和命令序列必须在十层链的每一层产生相同 actor 数量、种类、位置、loot、RNG draw counter 和 state hash。

## 兼容性

- 协议升级到 1.49，用于拒绝以 1.48 预算前规则解释新楼层和回放。
- save 容器保持 v1，state hash 保持 Schema v19；现有投影已覆盖楼层、actor、item、RNG、地牢状态和 content hash。
- v48 content hash 被列为迁移来源。v48 已生成楼层保持原 terrain、actor、item 和 RNG，不按预算补生成；旧存档缺失的新压力地牢状态按“守护者未击败”补入。当前内容 hash 下非空但不完整的 dungeon state 集仍被拒绝。
- contract-v49 从 v48 迁移 96 个 exact fixtures，并新增浅层预算、深层主题切换和十层最终压力 3 个 fixture。active baseline 共 99 个 exact fixtures，0 waiver。

## 固定版本与规模

- 协议：1.49；
- 内容包：1.42.0；
- content hash：`5d65fd9ca827dd05fc035650b82046edb592d563565c7e4075b32512a43f4e1f`；
- 内容：terrain 37、actor 8、affix 1、item 5、encounter table 2、loot table 5、theme table 2、vault 2、world 1；
- save：v1；state hash：Schema v19；
- active baseline：99 exact fixtures，0 waiver。

## 明确遗留

- Vault 旋转、镜像、自由房间落位、多入口、大模板和多个 Vault 同层放置；
- 房间数量/形状预算、整层空间成本、区域内多主题拼接和连通性重试；
- friends、escort、pit、formation、pack AI、召唤、繁殖、种群上限和 unique 过滤；
- 预算化陷阱、门、特殊地形与任务目标分配；
- 分支、shaft、随机楼梯、同层多个连接点与独立到达点；
- 专门的性能计时基线；本切片锁定规模和确定性状态，不把机器耗时写入规则契约。
