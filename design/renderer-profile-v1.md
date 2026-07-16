# 大地图渲染 Profile v1

状态：Windows Tauri/WebView2 基准和首轮动态视图优化已实现并接入 CI

## 1. 目的与边界

本基准用于回答三个问题：

1. 当前 PixiJS 分层后端在接近大型地下城尺寸时的初始化和更新成本；
2. 8、16、32 格 terrain chunk 的实际取舍；
3. 动态层应继续整图预分配，还是改为按可见 chunk 复用。

基准场景只存在于 TypeScript 开发诊断代码，不进入正式 `.rfbcontent`，不会改变 content hash、存档、回放或 Rust 规则。普通启动不会注册 profile 全局钩子；Windows WebDriver E2E 通过本地存储显式启用后调用。

## 2. 原创压力场景

`rfb-render-profile-large-original-v1` 使用完全原创且确定性的 192×64 地图，共 12,288 格：

- 边框和稀疏规律墙体；
- 可见、记忆和隐藏三种状态；
- 稀疏物品、角色和双色光照；
- 256 格动态层更新；
- 96 格跨地图地形更新；
- 32 次玩家居中镜头扫描；
- ASCII 到图片 tileset 的全量热切换；
- 操作完成后的 45 个 `requestAnimationFrame` 间隔样本。

每次运行依次创建 8、16、32 格 chunk 的独立 `PixiRendererBackend`，避免一个配置的缓存和对象污染下一个配置。

## 3. 输出

本地 `npm run e2e` 会写入被 Git 忽略的 `test-results/render-profile.json`。Windows CI 成功时单独上传 `tauri-render-profile` artifact。报告 Schema v1 包含：

- 初始化、初始可见视图分配、完整快照、镜头扫描、动态更新、地形更新和 tileset 切换耗时；
- Canvas 物理像素尺寸和设备像素比；
- chunk 总数、可见数和累计重建数；
- active/pooled dynamic chunk、实际 cell view、实际动态 Pixi display object 和整图理论对象数量；
- 帧间隔 median、p95 和 max；
- 基于对象规模给出的架构建议。

耗时受机器、GPU、WebView2 和刷新率影响，因此 CI 目前只校验结构、有限值、对象计数与 chunk 关系，不设置易抖动的毫秒门槛。

## 4. 2026-07-16 Windows 优化前基线

环境：Windows WebView2，`devicePixelRatio = 1`，Canvas 为 5376×1792 物理像素。以下是第二次完整本地运行：

| Chunk | Chunk 数 | 可见数 | 初始化 | 完整快照 | 96 格地形更新 | Tileset 切换 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 192 | 16 | 171.4 ms | 208.9 ms | 30.3 ms | 203.5 ms |
| 16 | 48 | 4 | 133.1 ms | 144.2 ms | 53.2 ms | 180.3 ms |
| 32 | 12 | 4 | 111.5 ms | 171.1 ms | 98.7 ms | 182.3 ms |

三档都为 12,288 格预分配 86,016 个动态 display object。持续帧间隔 median 约 6.9 ms、p95 约 7 ms，说明当前主要风险不是稳定帧提交，而是整图初始化、全量重绘和长期对象占用。

## 5. 可见 Chunk 复用后的复测

`pixi-layered-chunks-v3` 保留 12,288 格语义数据，但玩家居中模式只挂载可见 chunk 的动态视图。相同机器和场景的首次完整复测如下：

| Chunk | Active chunk | 动态对象 | 初始化 | 初始视图 | 完整快照 | 32 次镜头扫描 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 16 | 7,168 | 50.7 ms | 5.3 ms | 154.0 ms | 29.0 ms |
| 16 | 4 | 7,168 | 29.7 ms | 8.6 ms | 112.1 ms | 23.7 ms |
| 32 | 4 | 28,672 | 28.8 ms | 32.7 ms | 107.6 ms | 67.1 ms |

16 格模式的动态对象比整图理论值减少约 91.7%，初始化从优化前约 133 ms 降到约 30 ms，完整快照从约 144 ms 降到约 112 ms。镜头扫描现在包含视图释放、复用、局部重定位和内容刷新，因此从接近零增加到 32 次约 24 ms，平均每次仍低于 1 ms。32 格虽然 terrain chunk 少，但单个动态 view 和地形重建都明显更重，继续证明 16 格是当前较均衡的默认值。

## 6. 决策

- 正式默认 terrain chunk 从 8 调整为 16；
- 16 格把大型场景 chunk 数减少 75%，完整快照明显快于 8，同时避免 32 格在局部地形更新时重绘过大区域；
- 不实现普通逐 sprite 池。`visible-chunk-reuse-v1` 已按完整 chunk view 复用 object/actor/visibility/lighting 对象；
- 后端保留每格最新语义数据，dirty update 即使发生在不可见 chunk 也不会丢失；该 chunk 再次进入视口时会从权威前端语义缓存完整刷新；
- 整图模式继续挂载全部 chunk，保证现有 HTML 滚动行为；从整图切回玩家居中时只为每种边缘尺寸保留一个空闲 view，其余对象立即销毁；
- CI 现在要求实际动态对象数低于 86,016 整图理论值，并固定 8/16/32 三档的可见容量关系。

## 7. 风险与后续测量

- 高 DPI 下 5376×1792 Canvas 会按设备像素比放大，未来应记录最大纹理尺寸并评估视口大小 renderer；
- 首个帧样本会包含 tileset 重建后的长帧，报告保留 max 但不把它误认为稳定帧率；
- 当前镜头扫描是同步重绑定；若未来动画或连续平滑滚动造成抖动，再评估分帧预热相邻 chunk，不提前增加复杂度；
- resize、最小化/恢复、DPI 切换和整图滚动矩形虚拟化仍需独立场景。
