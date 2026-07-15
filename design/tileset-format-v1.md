# Tileset manifest 与资源回退 v1

状态：ASCII glyph atlas、图片 atlas、manifest v1 和回退链已实现

## 1. 边界

Rust 核心只输出地形与角色的语义 ID。tileset 属于非权威前端资源，不能影响命令、RNG、存档或 state hash。

ASCII 与图片模式共用同一条 PixiJS `Sprite + Texture` 渲染路径：

- ASCII 字符先绘制到动态 canvas atlas，再切成 PixiJS texture；
- 图片资源加载为 atlas texture，再按 manifest 坐标切片；
- 地图 cell 始终持有保留式 sprite，只替换 texture、tint 和背景；
- tileset 切换只重绘已有 cell，不重新创建游戏会话。

## 2. Manifest v1

Schema 位于 `schemas/tileset-v1.schema.json`。manifest 至少包含：

```json
{
  "format": "rfb-tileset",
  "formatVersion": 1,
  "id": "rfb.tileset.example",
  "mode": "ascii",
  "tileWidth": 28,
  "tileHeight": 28,
  "mappings": {},
  "fallback": {
    "glyph": "?",
    "foreground": "#ff77aa",
    "background": "#2b1522"
  }
}
```

图片模式额外声明安全相对路径 `atlas.source`、columns 和 rows。运行时拒绝远程 URL、绝对路径、路径穿越、未知字段、越界 tile 坐标和非法颜色。

## 3. 回退顺序

每个语义 ID 按以下顺序解析：

1. 图片模式且图集成功载入、mapping 存在 tile：使用图片切片；
2. mapping 提供 glyph：使用 glyph atlas；
3. 原创内容定义提供 glyph：使用内容 fallback；
4. ID 未知：使用 manifest 的醒目 `?` fallback。

整个图片图集加载失败或尺寸不足时，所有映射自动进入同一 glyph 路径。单个 mapping 缺少 tile 时只回退该对象，不影响其他图片 tile。

## 4. 当前资源

- `rfb.tileset.ascii-default`：动态 glyph atlas；
- `rfb.tileset.image-demo`：原创三格 SVG atlas，余烬微粒故意不提供图片 tile，用于持续验证逐项回退；
- 内容 glyph 直接从 `packs/rfb-demo-original` 构建期导入，避免维护第二份字符表；
- 玩家可以在运行中切换两种预设，选择保存于前端本地设置。

## 5. 测试与后续

Node 测试覆盖两份已提交 manifest、图片/字符选择、图集缺失、未知 ID 和不安全路径。Windows Tauri E2E 已覆盖 ASCII/图片热切换、同一 Canvas 保留和 400 格重绘计数。后续仍需：

- tileset 稳定视觉基准与截图差异测试；
- 任意 tile 尺寸缩放与高清 atlas；
- tileset 包签名、来源审计和用户安装目录；
- 动画 tile、自动连接地形和独立特效图集；
- 内容包运行时加载后，从已编译内容索引取得 glyph，而不是构建期 JSON import。
