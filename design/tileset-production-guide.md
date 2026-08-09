# Tileset 生产与接手指南

状态：适用于当前 RFB Pixel 28，也可作为其他固定网格项目的起步模板

本文说明如何从内容清单制作、接入和验收图片 tileset。Manifest 的运行时契约见 [`tileset-format-v1.md`](tileset-format-v1.md)，当前图集的色板、视觉规则和固定坐标见 [`../assets/tilesets/rfb-pixel-28/STYLE.md`](../assets/tilesets/rfb-pixel-28/STYLE.md)。

## 1. 先理解边界

Tileset 是前端表现资源，不是游戏规则数据：

- Rust 核心输出地形、物品和角色的稳定语义 ID；
- 前端 manifest 把语义 ID 映射到图片或 fallback glyph；
- 换图、减色和移动光影不能改变命令、RNG、存档或 state hash；
- terrain、object、actor、visibility、lighting 是独立渲染层；
- 黑暗、记忆区、FOV 和动态光照由运行时处理，不能烘焙进素材。

当前地图的逻辑格固定为 28×28 像素，2× zoom 使用最近邻显示为 56×56。高分辨率图片即使通过独立 `image` mapping 载入，也会在地图上缩放到一个逻辑格；需要大幅角色立绘时，应建立独立的 UI 美术通道，而不是扩大地图 tile。

## 2. 当前目录与权威文件

```text
assets/tilesets/rfb-pixel-28/
├── STYLE.md                    # 美术规范、色板和固定坐标
├── rfb-pixel-28.aseprite       # 主图集唯一可编辑源文件
└── players/
    └── warrior.aseprite        # 玩家独立素材源文件

web/public/tilesets/rfb-pixel-28/
├── atlas.png                   # 主图集运行时导出物
├── tileset.json                # 语义 ID → 图片坐标
└── players/
    └── warrior.png             # 玩家独立运行时图片

schemas/tileset-v1.schema.json  # manifest 机器可读契约
web/src/tileset-manifest.ts     # manifest 严格解析与回退
web/src/tileset-runtime.ts      # PixiJS 图片加载与切片
web/src/tileset-manifest.test.ts
```

维护原则：

1. `.aseprite` 是美术源，PNG 是导出物；不要直接在 PNG 上继续编辑。
2. `STYLE.md` 是当前 tileset 的坐标和视觉权威；本文只描述通用流程。
3. `tileset.json` 使用内容包中的稳定语义 ID，不使用显示名称或临时序号。
4. 美术工作标签不是本地化名称。RFB 内容的中文显示名仍以项目规定的权威 RFB `master` 数据为准。
5. 已发布坐标只追加、不重排；多个语义 ID 可以有意识地复用同一图块。

## 3. 当前技术规格

RFB Pixel 28 当前规格：

- 单格：28×28 RGBA；
- 主图集：8 列 × 16 行，即 224×448；
- 帧数：1；
- 图层：单一 `Artwork`；
- 地形允许铺满单格；actor 和物品通常保留至少 2px 透明边距；
- 普通有效像素只使用 alpha 255，背景使用 alpha 0；
- 单格通常使用 6–12 色，简单轮廓可以更少；
- 不使用抗锯齿、半像素、手绘半透明阴影或缩放笔刷；
- 统一左上方光源。

换用其他项目时，可以重新选择 tile 尺寸和列数，但必须同时统一源画布、manifest、地图逻辑格和测试；只改 atlas 分辨率不会提高地图上的实际显示尺寸。

## 4. 一轮素材生产流程

### 4.1 选择内容

先从正式内容定义中列出仍在 glyph fallback 的语义 ID，再按实际出场频率和场景聚类。每轮优先填满整行，避免频繁扩画布。

为每格记录：

- 稳定语义 ID；
- 权威显示名称；
- 类型：terrain、object 或 actor；
- 主要轮廓和主色；
- 是否可以复用已有素材；
- 目标 `(x, y)`。

不要仅因名字相近就复用。具名角色、体型差异明显或战斗职责不同的对象，应单独绘制；只有在 1× 下确实无法从额外细节获益时才复用。

### 4.2 冻结坐标和像素

编辑前确认 Aseprite 源文件与当前 atlas 逐像素一致，并保存当前像素基线。扩展时：

1. 保持列数不变；
2. 以完整的 28px 行扩展画布；
3. 更新 manifest 的 `atlas.rows`；
4. 在 `STYLE.md` 分配新坐标；
5. 确认旧区域零像素变化；
6. 未分配的新格保持完全透明。

不要在脏工作树中拿 `HEAD` 当作当前像素基线；应以编辑开始时的 `.aseprite` 和 PNG 为准，在内存或临时副本中比较。

### 4.3 使用 ImageGen 制作概念参考（可选）

ImageGen 适合探索轮廓、材质和配色，不适合直接成为 28×28 成品。推荐一次生成一整行参考，并固定顺序：

```text
制作 8 个横向排列的奇幻 roguelike 像素概念角色。
每个角色按 28×28 的可读性设计，硬边、无抗锯齿、左上光源、
4–10 色、至少 2px 留白。严格按以下顺序：……
不要文字、网格、场景、地面、投影、光晕或额外物体。
```

参考图必须经过人工重绘：

- 提取大轮廓和少量关键特征；
- 删除生成图中的渐变、噪声和近似色；
- 把材质压缩为 3–5 级明暗；
- 用单像素修正眼睛、四肢、裂缝和边缘；
- 在深色、浅色地形上同时检查轮廓。

不要直接把概念图缩到 28×28 后提交。直接缩小通常会产生数百种近似色、模糊边缘、不可控半透明和无法维护的细碎噪声。缩小稿最多只能作为构图底样。

### 4.4 在 Aseprite 中绘制

1. 开启 28×28 网格和像素对齐。
2. 使用 1px 铅笔；关闭抗锯齿。
3. 先画深色轮廓和纯色剪影，在 1× 下确认辨识度。
4. 再加入中间色、高光和少量材质细节。
5. actor、物品与透明背景之间不得残留杂色边缘。
6. 不绘制地面投影、FOV、火把色或运行时黑暗。
7. 同一行完成后再统一检查比例和光源。

玩家变体不进入主 atlas。每个玩家形象使用独立的 28×28 Aseprite 和 PNG，通过 manifest 的 `image` 字段引用；主 atlas 中已经冻结的玩家保留格不得挪用。

### 4.5 导出 PNG

使用 Aseprite 批处理导出完整画布，不裁边、不旋转、不添加格间距：

```powershell
$aseprite = 'D:\Games\software\aseprite\build\bin\aseprite.exe'
& $aseprite --batch `
  'assets/tilesets/rfb-pixel-28/rfb-pixel-28.aseprite' `
  --save-as 'web/public/tilesets/rfb-pixel-28/atlas.png'
```

其他机器只需替换 `$aseprite`。导出后检查：

```text
atlas width  = columns × tileWidth
atlas height = rows × tileHeight
```

PNG 编码字节可以因工具版本变化；验收应比较解码后的 RGBA 像素，而不是要求 PNG 文件 hash 与临时导出物相同。

## 5. 配置 manifest

### 5.1 主 atlas 图块

```json
"demo.actor.floating-eye": {
  "foreground": "#62abd4",
  "tile": { "x": 0, "y": 14 }
}
```

`foreground` 是图片失效时的 glyph 颜色；图片正常加载时保持原始颜色，不使用 tint 改色。

### 5.2 复用图块

多个对象可以指向同一坐标：

```json
"demo.actor.crow": {
  "foreground": "#91a0ad",
  "tile": { "x": 0, "y": 6 }
},
"demo.actor.crow-of-durthang": {
  "foreground": "#91a0ad",
  "tile": { "x": 0, "y": 6 }
}
```

复用关系必须写入 `STYLE.md`，便于以后判断这是有意设计还是漏画。

### 5.3 独立图片

```json
"demo.actor.warrior-player": {
  "foreground": "#9bf0fd",
  "image": "players/warrior.png"
}
```

独立图片路径相对于 `tileset.json`，必须是安全相对路径，尺寸至少等于 tile 尺寸。同一个 mapping 不能同时包含 `tile` 和 `image`。

### 5.4 回退

当 atlas、独立图片或单个 mapping 缺失时，运行时会依次使用 mapping glyph、内容 glyph 和 manifest 的醒目 `?`。不要为了隐藏缺图而随意复制无关 tile；明确的 glyph fallback 比错误图片更容易发现和修复。

## 6. 自动验收

每轮至少验证以下条件：

- Aseprite 导出与提交的 PNG 逐像素一致；
- 旧坐标区域零像素变化；
- atlas 尺寸与 manifest 行列完全匹配；
- 每个新 tile 非空且没有越过自己的 28×28 格；
- actor 和物品保留约 2px 透明边距；
- 普通素材只含 alpha 0 或 255；
- 单格颜色数量符合 `STYLE.md`；
- 未分配格保持透明；
- manifest 中的坐标未越界，`tile` 与 `image` 不冲突；
- 新语义 ID 坐标和有意复用都有测试断言。

相关自动检查：

```powershell
Set-Location web
node --test src/tileset-manifest.test.ts
npm run build:ui
Set-Location ..
git diff --check
```

日常素材扩展不需要运行完整桌面 E2E。只有修改 manifest 契约、纹理加载、地图缩放或渲染分层时，才扩大到相关渲染器和桌面测试。

## 7. 视觉验收

自动检查不能代替游戏内目视检查。至少检查：

1. 1× 下轮廓和对象类别是否立即可辨；
2. 2× 下是否保持硬边，没有模糊或格缝；
3. 明亮、昏暗、记忆区和视野边缘是否仍可辨认；
4. actor 和物品叠在不同地形上是否清楚；
5. 透明区域是否正确显示下方地形；
6. 未绘制内容是否正确回退到 glyph；
7. 同一行的相近对象是否能通过轮廓而不只是颜色区分。

视觉验收失败时优先改轮廓、明度和占格比例，不要先增加颜色和微小纹理。

## 8. 常见问题

### 2× zoom 模糊

确认纹理使用最近邻、画布坐标为整数，并且素材没有半透明抗锯齿边缘。不要制作第二套 56×56 atlas 来绕过缩放问题。

### 图块出现接缝

确认导出没有 padding、spacing 或裁边；地形边缘必须正好落在 28px 网格上。检查 atlas 实际尺寸是否等于 manifest 声明尺寸。

### 图片正常但游戏仍显示 glyph

依次检查语义 ID、manifest 路径、JSON 解析、atlas 尺寸、坐标范围和浏览器网络加载。独立图片加载失败只影响对应 mapping；主 atlas 失败会使所有 atlas tile 回退。

### 相近怪物混在一起

先改变剪影：体高、头部方向、肢体数量、身体分节和重心。颜色只能作为第二识别通道。

### Aseprite 与 PNG 看起来相同但检查失败

用 Aseprite 重新导出临时 PNG，再比较解码后的 RGBA 像素。不要比较压缩文件字节，也不要让图像编辑器自动转换色彩模式。

## 9. 交接清单

接手者开始下一轮之前应能回答：

- 哪个 `.aseprite` 是唯一源文件；
- 当前最后一个冻结坐标和下一可用行；
- 本轮语义 ID 来自哪里；
- 哪些 mapping 是有意复用；
- atlas 行列与实际像素尺寸；
- 如何导出、运行测试和进入游戏做 1×/2× 验收；
- 哪些素材仍处于 glyph fallback。

提交前应留下：

- 更新后的 Aseprite、PNG、manifest 和 `STYLE.md`；
- 新坐标与复用关系的测试；
- 通过的像素检查、manifest 测试、类型检查和生产构建结果；
- 尚未完成的视觉验收或已知回退清单。

## 10. 迁移到其他项目

新项目可以复用这套生产顺序，但不应直接复制 RFB 专用坐标或美术：

1. 先确定逻辑 tile 尺寸、渲染层和 zoom 规则；
2. 建立一份项目自己的 `STYLE.md`；
3. 用地板、墙、玩家、物品四格完成最小切片；
4. 接入稳定语义 ID、manifest 和 glyph fallback；
5. 通过 1×/2×、透明叠层、FOV 和光照验证后再扩充；
6. 按 [`licensing-and-assets.md`](licensing-and-assets.md) 审核第三方参考和素材，不复制授权不明的旧 tileset。

最小切片是必须的：它能在大量绘制之前暴露格尺寸、透明层、缩放、坐标和 manifest 设计错误。
