<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v154: Warrens surface and entrance interaction

状态：active baseline。协议保持 `1.123`，save 容器保持 v1，state hash Schema 保持 `55`。demo 内容包升至 `1.144.0`，content hash 为 `25c0ed9c6afd24e3f74cdf3bae09f60044daec3b6ae9149f86a9e530c30087db`；active baseline 继续包含 456 条 exact fixture、零 waiver。

## 问题

contract-v153 已生成随机 Warrens 地牢层，但生产旅程的初始 `demo.floor.surface` 仍使用室内石地/石墙视觉，通用楼梯文案还残留 Echo Depths 与 Original Lab 名称。角色走上连接后不会自动换层，而界面只提供 `<` / `>` 键盘说明；到达随机楼层时角色又恰好站在返程楼梯上，导致地表、入口、返程楼梯和继续下行楼梯容易被混淆。

## 实现

- 地表保持独立、固定、可返回的 `demo.floor.surface`，不消耗 RNG，也不改变九层 Warrens 的 seeded 生成与 stored-floor 生命周期。
- 新增草地、土路、洞口岩壁与密林四个独立 terrain。地表以草地填充、密林封边，洞口附近使用土路、岩壁和树丛形成可识别入口区；玩家、两件地表补给与入口坐标保持不变。
- 通用上下楼梯名称保持稳定，描述改为不绑定具体旧世界的地下/上一层语义，移除 Echo Depths 与 Original Lab 残留。
- 地牢信息在地表显示“地表”且隐藏深度，不再把地表伪装成 Warrens 深度 0；未击败 Boss 仍按权威 campaign 状态显示。
- 地图工具栏新增状态感知的连接按钮。未站到连接上时按钮直接说明需要站到 `>` 或 `<`；站在地表入口、上行楼梯或下行楼梯时分别显示“进入兽穴”“上楼”“下楼”，点击后发送既有 `TraverseStairs` 命令。
- 查看模式现在同时报告可见格的 terrain 名称；地表入口指引明确入口在起点南侧一格，并同时说明按钮与 `Shift + .` 操作。
- ASCII 与演示图像 tileset 都为四种地表 terrain 提供显式样式，不落入未知内容的问号回退。
- Webdriver 技术回归继续使用 Original Lab，但其 feature 构建被隔离到 `target/e2e`；它不再覆盖供开发者和玩家手动启动的正常 `target/debug/rfb-tauri.exe`。
- Warrens 继续使用地牢默认的 `reset-on-surface` 实例生命周期：返回地表会结束并丢弃旧 Warrens 实例，再次进入时分配新实例并重新生成地图、怪物和掉落；玩家经验、背包与装备保留。

## 保持不变

- 不自动穿越楼梯，避免移动误触改变楼层。
- 不为刷怪另建特殊机制；将来的自然刷怪频率与规则直接采用 RFB 原版设计，当前资源循环以返回地表后重新进入为准。
- 不修改协议 DTO、命令、存档容器、state-hash 字段结构或 RNG 消耗。
- Warrens 深度 1–9 的生成参数、怪物/掉落、守护者、胜利返程和地表退休规则不变。

## 验收

- 核心测试从正常地表起点向南移动一格、使用入口并到达随机 Warrens 一层；另以已清空怪物的一层验证返回地表会丢弃旧实例、重入会分配新实例并生成新怪物。既有 16-seed 连通/差异/返程矩阵与首领—胜利—退休闭环继续通过。
- 前端测试固定地表入口、上行和下行按钮状态，地表位置展示以及中英文键集合。
- 内容编译器固定 53 terrain、包 `1.144.0` 和上述 content hash；旧 `1.143.0` hash 进入兼容列表。
- 456 条 exact fixture 因内置 content hash 变化刷新权威 state hash；fixture 456 同时继续固定正常地表入口与 seed 42 随机一层。
