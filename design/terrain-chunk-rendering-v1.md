# 静态地形 Chunk 渲染 v1

状态：PixiJS 桌面垂直切片已实现

## 1. 目标

大地图不能继续为每个静态地形格长期保留一组可直接提交的 `Graphics + Sprite`。本阶段把地形层改为固定大小 chunk 的 `RenderTexture`，并在玩家居中镜头中跳过视口外 chunk，同时保持以下边界不变：

- Rust 仍权威决定地形、FOV、探索记忆和光照；
- 物品、角色、可见性和光照仍是独立动态层；
- `RenderWorld` dirty cells、存档、回放和 state hash 不受缓存影响；
- ASCII 与图片 tileset 使用同一条 terrain chunk 路径；
- 整图模式仍允许 HTML 容器滚动，不在前端滚动区域之外错误剔除地图。

## 2. Chunk 布局

当前默认 `TERRAIN_CHUNK_SIZE = 16` 格。该值来自 192×64 Tauri/WebView2 实测；profile 仍可显式创建 8、16、32 格后端进行对比。地图按行优先切分，边缘 chunk 会裁剪到实际尺寸：

```text
20×20 地图
  ↓
2×2 chunk
  ↓
16×16, 4×16, 16×4, 4×4
```

20×20 原创测试地图因此有 4 个 terrain chunk。chunk 索引和可见集合由无 PixiJS 依赖的纯函数计算，便于 Node 单元测试和未来其他 backend 复用。

## 3. 地形缓存

`PixiRendererBackend` 为每个 chunk 长期保留一个 terrain sprite。重建时临时创建该 chunk 内的地形背景和 glyph/tile，使用 PixiJS `renderer.generateTexture()` 生成 nearest-neighbor `RenderTexture`，随后销毁临时 display objects。

这使正常帧中的地形提交从“每格背景和 sprite”变为“每个可见 chunk 一个 sprite”。动态层不被烘焙进地形纹理，避免角色移动、物品拾取、FOV 或光照变化导致静态缓存失效。

后端保存每格最后一个 `terrainId`：

- 初始快照重建全部 chunk；
- 普通等待重建 0 个 chunk；
- 玩家移动和 FOV/光照 delta 重建 0 个 chunk；
- 物品拾取重建 0 个 chunk；
- 真正的地形 ID 改变只重建涉及的 chunk；
- tileset 热切换明确使全部 terrain chunk 失效并重建。

旧 RenderTexture 在替换时立即销毁 GPU source，防止反复切换 tileset 累积显存。

## 4. 动态层分组与剔除

五个语义层保持不变：terrain、object、actor、visibility、lighting。

object、actor、visibility 和 lighting 的逐格对象按相同 chunk 归入子容器。玩家居中模式根据相机偏移、缩放和 420×420 视口，把屏幕矩形转换回未缩放世界坐标，并额外保留一格 overscan；完全不相交的 chunk 会同时隐藏五层对应对象。

整图模式关闭 chunk 剔除。原因是该模式可能由 `.map-host` 自身滚动，而 Pixi 相机偏移保持为零；在没有监听滚动位置的情况下进行剔除会让滚动后的远端地图缺失。

## 5. 诊断属性

`#map-host` 暴露以下只读 E2E/性能诊断数据：

- `terrainMode = chunk-render-texture-v1`；
- `terrainChunkSize` 和 `terrainChunkCount`；
- `visibleChunkCount` 和 `culledChunkCount`；
- `lastRebuiltTerrainChunks` 和 `totalRebuiltTerrainChunks`。
- `rendererCellViewCount` 和 `rendererDynamicDisplayObjectCount`。

后端 ID 升级为 `pixi-layered-chunks-v2`。这些属性只用于测试和性能分析，不进入游戏协议或规则状态。

## 6. 验证

Node 测试覆盖 20×20 地图的 4 个边缘裁剪 chunk、cell 索引、整图零剔除、玩家在边缘/中部时的 1/2 个可见 chunk，以及 150% 缩放后的世界坐标换算。

Windows Tauri E2E 验证初始快照重建 4 个 chunk；等待、移动、拾取和存档恢复不重建 terrain chunk；镜头和缩放只改变可见/剔除计数；tileset 热切换重建 4 个 chunk；Canvas 身份保持不变。E2E 还运行 192×64 原创压力场景并输出 [大地图渲染 Profile v1](renderer-profile-v1.md)。

本地视觉截图可通过以下命令生成到被 Git 忽略的 `test-results/tauri-e2e-success.png`：

```powershell
$env:RFB_E2E_CAPTURE_SCREENSHOT="1"
cd web
npm run e2e
```

## 7. 当前限制与后续

- 16 格是当前基线，不作为永远不变的公开格式；
- 不可见 chunk 的动态状态仍会同步更新，而且全部 7 个逐格 display object 仍会预分配；
- 真正的地形动画需要进入独立 effects pass，不能每帧使静态 chunk 失效；
- 整图滚动模式尚未按滚动矩形剔除；
- profile 已决定下一步采用可见 chunk 动态视图复用，而不是普通逐 sprite 池；低分辨率 lighting RenderTexture 和 Debug pass 后续再评估。
