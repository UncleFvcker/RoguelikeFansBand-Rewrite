# 桌面分层 RendererBackend v1

状态：首个 PixiJS 分层 backend 已实现

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

当前测试世界没有正式 FOV/探索规则，因此运行模式明确标记为 `all-visible`。`remembered` 和 `hidden` 已有独立 render delta 接口及测试，但不会由前端自行推断或启用；正式 FOV 必须由 Rust 规则层提供权威状态。

首版光照是 `presentation-player-v1`：围绕玩家的纯视觉阅读光，只根据玩家位置计算，不读取地形颜色，不进入存档、回放、RNG 或 state hash，也不代表火把距离、潜行或怪物视野。未来玩法光源应通过独立协议数据替换该策略。

玩家移动时，只重算旧/新光照足迹的并集。20×20 测试地图从 `(3,3)` 移到 `(4,3)` 更新 90 格，而不是整张 400 格地图；等待命令仍更新 0 格，拾取只更新 1 格。

## 4. 颜色与 tileset

- ASCII glyph 与图片 tileset 继续通过同一 `TilesetRuntime`；
- glyph 前景在绘制前经过 WCAG 风格对比度保护，防止前景和背景收敛；
- 图片 tile 保留原图颜色，不强制套用 glyph 对比度修正；
- 光照和可见性是独立 Graphics 层，不写回 tileset mapping 或地形背景；
- tileset 热切换重绘当前 400 个 RenderCell，但保留同一个 Canvas 和游戏会话。

## 5. 测试

Node 测试覆盖：

- 对比度保护；
- 地形语义不参与阅读光计算；
- 地形、物品和角色语义层同时存在；
- 玩家移动只产生有限且无重复的光照 dirty cells；
- 记忆 mask 使用独立 delta。

Windows E2E 验证 backend ID、五层顺序、90 格移动更新、0 格等待更新、1 格拾取更新、400 格 tileset 重绘，以及语言/tileset 切换时 Canvas 保持不变。

## 6. 后续

- 由 Rust 协议提供正式可见性、记忆状态和玩法光源；
- 将静态地形按 chunk 缓存为 RenderTexture；
- 增加 Effects、Interaction 和 Debug pass；
- 增加 resize、缩放、最小化恢复和截图差异测试；
- 根据性能分析决定 sprite pooling、GPU batching 与低分辨率 light RenderTexture。
