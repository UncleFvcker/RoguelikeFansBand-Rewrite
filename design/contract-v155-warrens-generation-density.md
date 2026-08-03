<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v155: Warrens generation density

状态：active baseline。协议保持 `1.123`，save 容器保持 v1，state hash Schema 保持 `55`。demo 内容包升至 `1.145.0`，content hash 为 `6af0e0500187c01f10612428b47ddc255ab415f6068dcd87800a17078fb534c2`；active baseline 继续包含 456 条 exact fixture、零 waiver。

## 来源结论

固定的 RFB v1.3.0.7 来源把 Warrens 声明为 1–9 层、66×22 的 `SMALLEST | CAVE | CAVERN` 地牢。小地图房间分配下限为五个，`CAVE` 将普通房间权重转给 fracave，生成结束后还绘制 magma/quartz streamers。怪物初始分配按面积缩放后仍有四次下限；这不是每层仅生成一只怪物。

## 实现

- rooms geometry 新增 `placement`：默认 `partitioned` 保持既有内容与 RNG 行为；`free` 在整个地图内部选择房间，并要求房间包围盒之间至少保留一格岩壁。
- 自由布局使用有界重试；内容验证同时要求最小房间仍能进入分区格，使极端失败时可确定性回退且不会生成无效楼层。
- Warrens 九层保持 66×22、五个房间、450 格房间预算、9:1 cavern/rectangle 权重、随机环形通道，以及 1–2 个上行和 4–5 个下行楼梯；仅把房间位置从固定 3×2 分区改为全图自由分布。
- 每层增加两个 streamer placement、总计 24 格，候选为可挖掘到普通地板的 magma vein 与 quartz vein；ASCII 和演示图像 tileset 都提供显式显示。
- Warrens encounter table 从一次提高到四次分配。深度 1–8 各保留四个普通 actor 槽；深度 9 使用五个槽，其中一个固定给 guardian，仍生成四个普通怪物。

## 保持不变

- 不复制原版地图字节、生成器代码、描述文本或完整怪物表；自由房间、cavern mask、走廊与 streamer 均为现有 Rust 生成框架内的独立实现。
- 不在本批加入 ambient 每回合刷怪。返回地表后重新进入仍会创建新 Warrens 实例，并刷新地图、怪物与掉落。
- 不为当前简化 Warg 补 `FRIENDS(3d3)`，也不扩展 guardian Schema 来表达原版 Mughash escort；这些需要连同完整 actor 等级与生态做独立内容批次。
- 地表、楼层生命周期、掉落、Boss 胜利、正常返程和退休流程不变。

## 验收

- 32 个固定种子的生成器单测固定五个房间、边界、面积预算、一格间隔和中心差异。
- 16 个旅程种子固定路线连通、同 seed 复现、楼层往返持久化，并要求任意两张一层地图至少有 120 个可行走格不同。
- 九层矩阵固定每层 24 格矿脉、深度 1–8 各四个普通怪物、最终层四个普通怪物加 guardian，以及全部上下楼梯返程。
- 内容编译器固定 55 terrain、包 `1.145.0` 和上述 content hash；旧 `1.144.0` hash 进入兼容列表。
