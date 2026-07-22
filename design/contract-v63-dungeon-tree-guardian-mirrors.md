# Contract v63：树状地牢与共享守护者镜像

状态：协议 1.63 / contract-v63 active baseline

## 原版依据

FrogComposband 用 `d_info[dungeon].final_guardian` 为整座地牢保存一个守护者种族；`alloc_guardian()` 在地牢底层按该共享身份生成，`dungeon_conquered()` 读取守护者种族的全局死亡状态，`_dungeon_boss_death()` 也按当前地牢的共享身份结算。`floors.c` 则把具体目标 `floor_id` 写入楼梯格，并为首次到达的新楼层分配稳定 ID。

contract-v63 保留“地牢拥有守护者身份、楼梯连接具体楼层、征服状态全局共享”三项语义。原版只有一个最大深度；多个最终叶层及其镜像是重构版在该模型上的扩展。

## 内容与拓扑

- `WorldDefinition` 新增独立 `dungeons`，每项声明稳定 `id`、`rootFloorId` 和共享 `guardianActorKindId`。
- 每座地牢必须形成一棵单根树：根楼层深度为 1；每个非根楼层恰有一个父楼层；同一父层的不同下行连接必须指向不同子楼层；禁止合流、循环、同层连接和跨地牢连接。
- 普通楼梯继续要求深度差 1，shaft 要求深度差 2；所有非地表连接仍需通过 `targetFloorId/targetConnectionId` 严格双向互引。
- 所有叶节点都必须声明 `finalFloor` 和 `guardian`，非叶节点禁止声明二者。最终层可以有多个并继续使用完整程序化生成管线。
- 同一地牢的所有最终层必须使用 `guardianActorKindId` 指定的同一 actor kind，但各自必须有不同的稳定镜像 `instanceId`。
- 旧 `nextFloorId/returnFloorId` 继续作为无显式连接楼层和历史存档的线性兼容输入；显式 `connections` 是分支树的权威边集合。

demo 回声地牢现在由根层分成普通左支、普通右支和跨两层 shaft 支路，并包含四个深度 3 最终叶层。不同楼梯不再汇入同一张下层地图。

## 运行时与征服

- 现有 `DungeonState.guardianDefeated` 继续作为地牢级唯一权威状态，不新增每层守护者状态集合。
- 未征服时，首次生成任意最终叶层都会生成该层的稳定镜像；已征服时，任何尚未访问的最终层都不再生成镜像。
- 击败任意镜像只在 `guardianDefeated: false -> true` 时产生一次 `dungeon.guardian-defeated`。事件保留实际击败位置的 `floorId` 和共享 actor kind。
- 同一事务会从所有离层状态删除其他已生成镜像及其携带物；被删除镜像不产生死亡事件、掉落或第二份征服奖励。实际被击败的镜像仍复用普通死亡与掉落管线。
- 存档校验遍历该地牢的全部最终层：未征服的已生成最终层必须保留镜像，已征服的当前层和离层都不得残留任何镜像。

## 兼容性与确定性

- 协议：1.63；内容包：1.56.0；content hash：`246f51864965fac494c7a39959f591caa0434d9fa4eac839501f9d09526eb617`。
- save 容器仍为 v1；`DungeonStateSaveDto.guardianDefeated` 可直接表达共享征服状态，不新增 DTO 字段。
- state-hash 继续为 Schema v23；树连接、当前层、离层镜像与清理结果已经由既有连接、actor、item、stored floor 和 dungeon state 哈希字段覆盖。
- v62 及更早存档继续迁移。若已生成楼层携带的旧连接集合不再匹配 v63 树定义，只清除连接索引并使用原 terrain 楼梯标签回退；地图、actor、item 和 RNG draw counter 均保持不变。

## Fixtures

- contract-v63 从 v62 刷新 125 个 exact fixtures。
- 新增同一父层两座下行楼梯进入不同最终叶层、不同镜像 ID 和存档往返场景。
- 新增先保存一个最终层镜像、再击败另一镜像并验证共享征服事件与确定性清理的场景。
- 内容与核心单元测试另覆盖双父/合流拒绝、守护者种类不一致拒绝、未访问镜像抑制和 v62 旧连接集合回退。
- active baseline 共 127 个 exact fixtures、0 waiver。

## 后续

下一切片推进 Vault 多入口、大模板连通性证明与跨走廊拼接。可重复使用同一楼层模板生成多个运行时实例、楼层淘汰和更一般的动态探索树仍保留为后续扩展，不属于本切片的稳定内容树边界。
