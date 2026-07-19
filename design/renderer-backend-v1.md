# 桌面分层 RendererBackend v1

状态：PixiJS 分层 backend v3 已实现，包含静态地形 chunk 缓存与可见 chunk 动态视图复用

## 1. 目标与边界

本阶段把地图渲染从单个 `MapRenderer`/单 sprite cell 拆成稳定的 RenderWorld 与 backend 边界，同时保持 Rust 规则、存档和 state hash 不变。

```text
GameSnapshot / GameUpdate
        ↓
    RenderWorld
        ↓ dirty RenderCell[]
 RendererBackend
        ↓
 PixiRendererBackend
```

`RenderWorld` 只消费协议 DTO，维护语义 cell、实体 kind 映射、视觉状态和 dirty 集合。`PixiRendererBackend` 只负责把合成输入写入长期存在的 GPU/Canvas 对象。未来 Canvas 或 WebGPU backend 不需要修改游戏核心。

## 2. 当前五层

1. `terrain`：地形背景与地形 glyph/tile；
2. `object`：地面物品背景与 glyph/tile；
3. `actor`：玩家和怪物背景与 glyph/tile；
4. `visibility`：可见、记忆、隐藏 mask；
5. `lighting`：独立彩色光与黑暗覆盖层。

物品与角色不再争用同一个 sprite。角色站在物品上时，物品层仍保留在角色层下方；角色移动或物品拾取只更新相关 cell。

## 3. 可见性与光照

Rust 从协议 1.3 起输出正式 FOV、探索记忆和逐格光照，当前协议版本为 1.11。运行模式标记为 `rust-fov-memory-v1`；`visible`、`remembered` 和 `hidden` 通过完整 `visualCells` 与增量 `changedVisualCells` 进入 RenderWorld，前端不再推断规则视野。

光照模式为 `rust-content-lights-v1`：玩家光源以及带 `light-source` 标签的怪物、物品由 Rust 使用整数强度生成。探索记忆会保存，但视觉输出不进入 RNG 或 state hash。

玩家和发光怪物移动时，核心只发送 FOV、记忆、实体或光照发生变化的视觉格。当前 20×20 E2E fixture 第一次移动合并更新 99 格，而不是整张 400 格地图；等待命令仍更新 0 格，拾取只更新 1 格。

## 4. 颜色与 tileset

- ASCII glyph 与图片 tileset 继续通过同一 `TilesetRuntime`；
- glyph 前景在绘制前经过 WCAG 风格对比度保护，防止前景和背景收敛；
- 图片 tile 保留原图颜色，不强制套用 glyph 对比度修正；
- 光照和可见性是独立 Graphics 层，不写回 tileset mapping 或地形背景；
- tileset 热切换重绘当前 400 个 RenderCell，但保留同一个 Canvas 和游戏会话。

## 5. 镜头与视口

地图提供两个可持久化的前端镜头模式：

- `full-map`：保持当前整图 Canvas 和窄窗口滚动行为；
- `player-centered`：使用约 15×15 格视口，玩家远离边缘时保持在视口中心，接近地图边缘时钳制世界偏移，避免显示地图外空白。

画面缩放提供 75%、100%、125%、150% 和 200% 五档，并与镜头模式一样写入本地前端设置。整图模式按比例改变整张地图尺寸；玩家居中模式保持 420×420 视口，缩放改变视口内可见的地图格数量。

五个视觉层共同挂在 PixiJS camera container 下。镜头移动只修改 container position，缩放只修改 container scale 并 resize 同一个 Canvas；两者都不重建 Canvas、sprite 或 tileset，也不产生新的 dirty cells。窗口或布局尺寸变化由 `ResizeObserver` 重新计算视口偏移；镜头模式、缩放和像素偏移只属于前端显示状态，不进入 Rust 核心、存档、回放或 state hash。

静态 terrain 层现按默认 16×16 格生成 RenderTexture。后端保存每格最新 `RenderCell`，object、actor、visibility 和 lighting 只为当前可见 chunk 创建动态视图；镜头跨 chunk 时按尺寸复用整组视图并重新绑定局部坐标。玩家居中模式使用一格 overscan，整图滚动模式则挂载全部 chunk，保证原有滚动行为。详细失效规则和诊断指标见 [静态地形 Chunk 渲染 v1](terrain-chunk-rendering-v1.md)。

## 6. 测试

Node 测试覆盖：

- 对比度保护；
- 地形语义不参与阅读光计算；
- 地形、物品和角色语义层同时存在；
- 玩家移动只产生有限且无重复的光照 dirty cells；
- 记忆 mask 使用独立 delta；
- 玩家居中、四边钳制、小地图居中和整图零偏移；
- 缩放后的中心跟随与远端边缘钳制。

Windows E2E 验证协议 1.11、`pixi-layered-chunks-v3` backend ID、`visible-chunk-reuse-v1` 动态模式、五层顺序和现有渲染/镜头/tileset 场景。

## 7. 后续

- 增加 Effects、Interaction 和 Debug pass；
- 为整图滚动模式增加滚动矩形虚拟化，避免超大地图在整图模式挂载全部动态视图；
- 扩展实际窗口 resize、缩放、最小化恢复和截图差异测试；
- 复测对象规模后再决定 GPU batching 与低分辨率 light RenderTexture。
