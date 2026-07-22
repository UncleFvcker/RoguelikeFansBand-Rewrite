# Contract v64：多入口 Vault 与连通拼接

状态：协议 1.64 / contract-v64 active baseline

Contract v64 完成 v63 留下的 Vault 拓扑缺口：一个模板可以声明多个边界入口；大模板落位后会为每个入口连接到既有走廊或可通行区域；候选只在整层连通证明通过时提交。它继续复用 v50 的旋转、镜像、自由 wall 区、多模板预算和确定性回退，不改变 save 容器或 state-hash Schema。

## 内容模型

`VaultDefinition` 的规范字段是 `entrancePositions`，数量限制为 1–8。每个位置必须位于模板边界、互不重复，并且不能被 wall 或其他不可通行 override 覆盖。旧的单数 `entrancePosition` 仍可读取；规范化阶段只在列表为空时把它迁移为一个入口，同时拒绝两个字段同时出现。宽高仍限制为 2–12，transform 列表必须去重。

内容加载器把 base terrain 和 overrides 展开为可通行格集合，从第一个入口执行固定 `north/east/south/west` BFS。所有可通行内部格都必须被访问，且每一个入口都必须属于该分量；否则整个内容包拒绝加载。这个证明防止“模板内部自成孤岛”或入口落在封闭装饰上的 Vault 进入运行时。

demo 内容包加入 8×8 的 `demo.vault.resonance-crossroads`，包含北、东、西、南四个入口、两个守卫候选和一份 loot。深度 8 的主题表把它作为加权候选，与 Crook、Spindle 和高权重但不可落位的 Monolith 一起验证多候选确定性。

## 生成与拼接

对每个 transform 和行优先 origin，生成器先检查整个模板 footprint 是否仍是 wall。随后按规范化入口顺序逐个计算朝外格：北、东、南、否则西。入口外部若已是可通行格，连接长度为零；若是 wall，则从该格开始对 wall 进行固定方向 BFS，直到遇到既有 connector 或可连接 terrain。每条新路径最多雕刻 12 格；越界、穿过 footprint、遇到不可连接地形或超预算均拒绝该候选。

候选验证在临时 terrain 副本上执行：先绘制模板和 connector，再收集所有 `walkable/open/bash/dig` 格，证明它们属于一个四向连通分量。失败候选不会修改真实 terrain、占位集合或 RNG 之后的预算；加权候选仍按 v50 的稳定抽取和后续候选回退。成功 placement 保存 connector cells，并将它们加入 terrain feature 保留集合；后续 Vault 也因 connector 已不再是 wall 而不能覆盖。楼梯、actor 和 loot 可以合法使用这些可通行格，但不会把走廊改回阻断地形。

这对应原版 FrogComposband 的结果约束：`rooms.c` 为 Vault 预留额外边界避免无入口模板，`generate.c`/`grid.c` 按房间中心链调用 `build_tunnel`/`build_tunnel2`。重构版不依赖隐式隧道副作用，而把“Vault 必须接入主网”变成可验证、可回放的内容与生成契约。

## 兼容性与确定性

- 协议 DTO、save v1 和 state-hash Schema v23 不增加字段；最终 terrain 已保存 connector 结果，离层 floor 也继续保存完整地形。
- v63 及更早存档不会回补多入口 Vault、connector 或新的 Vault actor/loot。载入已生成 floor 时保留 terrain、entity、item 和 RNG；只有当前内容连接集合不再匹配时才清除连接索引并回退 terrain 楼梯标签。
- 同一内容 hash、seed、初始存档和命令序列固定 transform、origin、BFS 路径、候选拒绝顺序、预算和 state hash。连接器不额外抽取 RNG。

## Fixtures 与验收

`tests/fixtures/contract-v64/scenarios` 从 v63 迁移 127 个 exact fixtures，并新增：

- `128-multi-entry-vault-stitching.json`：seed 93 在深度 8 生成 Crossroads + Crook，冻结四入口拼接、实体、terrain 和 save round-trip；
- `129-multi-entry-vault-transform.json`：seed 266 以不同入口前置命令生成 Crossroads + Spindle，冻结另一套确定性 transform/origin 和存档 hash。

active baseline 共 129 个 exact fixtures、0 个 waiver。`rfb-content` 的多入口规范化、重复/内部入口拒绝、断开模板拒绝，`rfb-core` 的大模板连通证明、空间失败回退和 v63 存档兼容测试必须全部通过。

## 明确不包含

v64 不实现同一模板的运行时多实例身份、跨楼层动态探索树、Vault 内可破坏墙体的运行时重连、任意多边形洞穴连接或多地牢并存实例。这些仍属于后续地牢实例与更一般连接图纵切。
