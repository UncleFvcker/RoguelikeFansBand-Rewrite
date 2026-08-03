<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v153: Warrens map generation

状态：历史 baseline；当前 active baseline 见 [contract-v154](contract-v154-warrens-surface-entry.md)。协议保持 `1.123`，save 容器保持 v1，state hash Schema 保持 `55`。本版 demo 内容包为 `1.143.0`，content hash 为 `4da783cfb282e4e2f2da517656ae5924e451083d0b67e6cf069887c840a2bfbe`；baseline 包含 456 条 exact fixture、零 waiver。

## 固定来源结论

本批次只读核对 FrogComposband/RFB v1.3.0.7 固定提交 `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`。Warrens 在 `d_info.txt` 中是深度 1–9、`SMALLEST | CAVE | CAVERN` 的地牢，使用 `FINAL_GUARDIAN_135`。`SMALLEST` 对应 66×22 屏幕尺寸；在这九层的深度范围内，整层 cavern 分支不会触发，`CAVE` 主要把普通房间替换为不规则 fracave 房间。常规生成约五个房间，打乱中心后用随机隧道连通，并尝试放置 1–2 个上行与 4–5 个下行楼梯；最终层没有下行。

旧源码、地图字节、heightmap/隧道算法、文本、成套数值表和素材均未复制。本实现只固定上述可观察行为，代码与数据结构独立设计。

## 新实现

- Warrens 地表与九层地牢统一为 66×22，满足当前 world 对所有楼层同尺寸的权威约束。
- 每层预算生成五个房间；形状权重为九份不规则 cavern、一份矩形，表达浅层 CAVE 地牢以不规则洞室为主、模板房间为辅的行为。
- cavern 房间从中心以有序候选、seeded frontier growth 雕刻出边界框面积的 5/8；每个洞室内部天然连通，精确格数进入房间预算。
- 洞室中心使用权威 RNG 做 Fisher–Yates 打乱，再组成闭环。每条通道逐步随机选择仍能缩短曼哈顿距离的横向或纵向步，避免固定 L 形通道并保留全图连通和冗余路线。
- 首个房间保留玩家入口；普通层的主下行位于最远房间，最终守护者也位于最远房间。洞室层不再强行放置旧 fallback 的固定中点秘密门。
- 深度 1–8 每层确定性生成 1–2 个上行与 4–5 个下行楼梯，深度 9 只生成 1–2 个上行。额外楼梯优先选择靠墙的可行走位置，避开陷阱、守护者、Vault/pit 和已占用的楼梯位置。
- 同一个 expedition seed 生成完全相同的地图与楼梯；不同 seed 产生不同布局。已经生成的楼层继续由现有 stored-floor 生命周期保存，往返不会重建。

## 明确保留的差异

本批次不复刻原版 fractal heightmap、房间尝试顺序、door/junction 概率、隧道穿墙启发式或精确 RNG 消耗。地貌用独立的连通 frontier mask 表达，通道用独立的单调随机步表达。现有 actor/loot 数量、战役拓扑、Warrior、首领、胜利返程和退休规则不变。

## 验收

- 固定 16 个 seed 检查每张深度 1 地图为 66×22、全图可行走地形连通、楼梯数量在声明范围内；至少 15 张 terrain 布局不同。
- 每个 seed 以第二个同 seed 会话证明 terrain 完全相同，并下行到深度 2 再返回，证明深度 1 terrain 没有重生成。
- 原有 16-seed 九层下行/九层返程矩阵和完整首领—胜利—返程—退休测试继续通过。
- fixture 456 固定 seed 42 从地表进入新生成的 Warrens depth 1，并固定 save round-trip 后的权威 state hash。
- 内容编译、Schema freshness、workspace tests、Clippy、replay、contract fixtures、Web/Tauri 与 Windows release/desktop E2E 使用仓库标准矩阵验收。
