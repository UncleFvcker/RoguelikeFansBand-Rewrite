# Contract v59：持久 pack identity 与首版 pack AI

## 目标

contract-v51 已能在生成时把 leader、friends 和 escort 放入 `cluster/ring` 阵形，但离开生成器后这些怪物只是独立 actor。contract-v59 为动态群体增加持久身份，并让成员在回合中按内容声明协同行动。

## 内容契约

- `EncounterGroupDefinition.packAi` 可声明 `leader`、`friends` 和 `escorts` 的行为；缺失时默认分别为 `seek`、`surround` 和 `guard-leader`。
- 首版 `MonsterPackBehavior` 只有 `seek`、`surround`、`guard-leader`。leader 不能声明 `guard-leader`；friends/escorts 可按角色使用任意首版行为。
- 生成的每个群体使用稳定的 `{floorId}.pack.N`，leader、friend 和 escort 保存同一个 pack ID。leader 的 `leaderId` 指向自身，成员指向该 leader。

## 持久状态

- `ActorSaveDto.pack` 保存 pack ID、leader ID、`leader/member` 角色和冻结行为；当前层与离层 `FloorSaveDto` 都通过 actor 列表保存。
- v58 及更早存档缺失 pack 字段时不重建旧楼层、不消耗 RNG，旧怪物继续使用独立 `seek`。
- 载入时拒绝非法 ID、重复/缺失 leader、跨 pack leader 引用、角色与 leader ID 不一致以及玩家携带 pack 状态。
- leader 死亡时剩余成员原子解散 pack，避免保存悬空 leader 引用；member 死亡不改变其余成员身份。
- pack 身份进入 state hash Schema v21；save 容器仍为 v1，新增字段均有空值兼容默认。

## 行动语义

1. `seek` 使用确定性寻路接近玩家并在相邻格攻击。
2. `surround` 按稳定 actor ID 取得包围槽位，行走到未占用的玩家相邻格；同一能量脉冲共享位置预留，避免两个成员选择同一格。无合法包围槽位时回退 `seek`。
3. `guard-leader` 优先保持与 leader 相邻并跟随 leader 的最新位置；leader 缺失时回退 `seek`。相邻玩家时仍先执行近战。
4. 所有寻路按固定八方向和 actor ID 顺序执行；同 pack actor 可作为规划中的通路节点，但第一步不能进入已占格，也不能穿过其他 pack。

## 原创场景与验证

- depth 6 ring 和 depth 7 cluster 的动态群体 fixture 现在显式锁定 pack ID、leader/member 角色和三种行为，并通过离层存档回环验证身份不丢失。
- Core 单元测试覆盖确定性包围槽预留、guard 跟随、leader 死亡解散、存档回环、非法 pack 拒绝和 v58 缺失字段兼容。
- active baseline 从 v58 迁移为 117 个 exact fixtures、0 waiver；contract-v59 只改变正式 content/state hash 版本和新增 pack 断言。

## 版本

- 协议：1.59；
- 内容包：1.52.0；
- content hash：`4cdcad204a7ccad6d67b8dcb50ccdcc188220a72d258c37219974fad51e5274d`；
- save：v1；
- state hash：Schema v21；
- active baseline：contract-v59，117 exact fixtures。

## 明确延后

- 怪物开门/破门、远程攻击选择、逃跑、召唤、繁殖、种群上限和 unique 过滤；
- 任意半径/模板 formation、跨房间群体和跨阻断区域连通性修复；
- 更复杂的阵营关系、气味/flow、特殊感知和 pack 间战术协作。
