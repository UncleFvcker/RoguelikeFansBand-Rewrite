# Rust 权威可见性与光照 v1

状态：功能自协议 1.3 实现，当前协议 1.18 继续使用 Rust FOV/探索记忆、内容标签光源和前端消费链路

## 1. 边界

Rust 核心负责输出当前可见、已经探索但当前不可见、从未探索三种状态，以及每格的确定性光照。TypeScript/PixiJS 只消费 DTO 并绘制独立 Visibility/Lighting pass，不再自行推断视野或围绕玩家生成临时阅读光。

相机、缩放、动画和颜色后处理仍属于前端显示状态；FOV 和光源输出不能改变命令结果、RNG、战斗、存档规则状态或 state hash。

## 2. 协议 1.3

```ts
type VisibilityState = "visible" | "remembered" | "hidden";

interface CellVisualDto {
  position: Position;
  visibility: VisibilityState;
  light: {
    color: number;
    intensity: number; // 0..100 integer
  };
}
```

`GameSnapshot.visualCells` 提供完整视觉重建数据；`GameUpdate.changedVisualCells` 只发送和上一 revision 不同的视觉格。地形/物品/角色语义变化继续使用 `cells`/`changedCells`，视觉变化不伪装成规则格变化。

## 3. FOV 与探索记忆

- 当前半径为 8 格；
- Rust 使用确定性整数 Bresenham 视线；
- `blocksSight` 地形阻挡其后的格子，但阻挡格自身可见；
- 当前 FOV 写入 `visible`；
- 曾经进入 FOV、当前离开的格写入 `remembered`；
- 从未进入 FOV 的格写入 `hidden`。

探索记忆作为 `SavePayloadV1.explored` 保存，以便载入后恢复地图记忆。旧存档缺失该字段时从空记忆开始并立即揭示当前 FOV。`explored` 在 state hash 输入中被省略，因此只改变地图记忆不会改变权威规则哈希或当前 contract 基准。

## 4. 光源 v1

光照强度使用 0..100 整数，避免跨平台浮点漂移。当前来源：

- 玩家暖色光，半径 6；
- 带 `light-source` 标签的角色，半径 5；
- 带 `light-source` 标签的地面物品，半径 4；
- 全图保留最低环境亮度。

多光源首版选择当前格最强的光源颜色和强度。后续 Lighting buffer 可改为加法/混色，但不能把结果写回地形定义或 state hash。

## 5. 前端规则

- `RenderWorld` 只应用 `visualCells`/`changedVisualCells`；
- 可见格显示当前物品和角色；
- 记忆/隐藏格不显示当前物品或角色，避免视觉遮罩泄露规则信息；
- hidden mask 完全不透明，remembered mask 保留地形记忆；
- 视觉 delta 与规则 changed cells 合并去重后提交 backend；
- 等待且视觉无变化时仍更新 0 格。

## 6. 当前测试

- 墙体自身可见、墙后格不可见；
- 玩家移动产生有限 `changedVisualCells`；
- 离开视野的已探索格进入 remembered；
- 发光怪物的内容标签生成独立光色；
- 探索记忆保存/载入一致，但不改变 state hash；
- 记忆格不暴露怪物和物品；
- Windows Tauri E2E 验证当前协议、400 个初始视觉格、初始可见/隐藏分布、移动后 remembered 格出现、99 格首次移动更新、0 格等待和 Canvas 复用。

## 7. 后续

- 把逐格 Graphics 光照升级为低分辨率 Lighting RenderTexture；
- 支持多光源颜色混合、遮光和地形自发光；
- 把视野半径和玩家光源参数迁移到角色/状态内容数据；
- 增加 FOV/光照 Debug pass；
- 大地图使用 chunk dirty tracking 和视口外剔除。
