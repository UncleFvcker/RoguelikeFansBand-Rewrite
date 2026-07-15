# RFB 内容数据格式 v1

状态：P0 格式基线已确定，Schema 尚未实现

## 1. 目标

怪物、物品、职业、种族、法术、地形、任务和视觉映射不再编译进巨型 C 结构体。内容定义与运行时实例分离，并满足：

- 稳定 ID；
- 可验证 Schema；
- 确定性加载；
- 本地化显示；
- 模组和数据包扩展；
- 存档可以记录精确内容集合；
- Rust、WASM 和 Tauri 使用同一份编译后数据。

## 2. 数据包结构

```text
packs/base/
├─ pack.json
├─ monsters/
├─ items/
├─ artifacts/
├─ races/
├─ classes/
├─ personalities/
├─ spells/
├─ terrain/
├─ quests/
├─ locales/
└─ assets/
```

`pack.json`：

```json
{
  "$schema": "https://rfb.example/schema/pack-v1.json",
  "formatVersion": 1,
  "id": "rfb.base",
  "version": "1.0.0",
  "titleKey": "pack-rfb-base-title",
  "dependencies": [],
  "loadAfter": [],
  "contentRoots": ["monsters", "items", "spells"]
}
```

源文件使用 UTF-8 JSON 和 JSON Schema。开发工具可以提供 JSONC 编辑体验，但进入构建和发布产物前必须转换为严格 JSON。

## 3. 稳定 ID

ID 格式：`namespace.category.name`，仅允许小写 ASCII、数字、点、下划线和短横线。

示例：

```text
rfb.monster.dragon.red
rfb.item.weapon.long_sword
rfb.spell.fire.fire_ball
rfb.terrain.wall.granite
```

规则：

- ID 是逻辑身份，名称由 Fluent key 提供；
- 已发布 ID 不得复用；
- 改名必须进入 alias/migration 表；
- 运行时实例引用定义 ID，不复制完整定义；
- 数组下标、英文显示名和中文译名都不能充当引用。

## 4. 定义与实例

内容定义描述固定规则：

```json
{
  "$schema": "https://rfb.example/schema/monster-v1.json",
  "id": "rfb.monster.dragon.red",
  "nameKey": "monster-red-dragon-name",
  "descriptionKey": "monster-red-dragon-description",
  "level": 60,
  "tags": ["dragon", "fire"],
  "stats": {},
  "abilities": []
}
```

运行时实例只保存定义 ID、实例 ID和动态状态。内容文件不能包含平台路径、Rust 枚举序号或图集坐标。

## 5. 验证与编译

构建工具 `rfb-contentc` 负责：

1. 解析严格 JSON；
2. 验证 Schema；
3. 检查重复 ID、悬空引用和依赖循环；
4. 检查数值范围和互斥字段；
5. 检查本地化 key；
6. 按稳定规则合并数据包；
7. 按 ID 排序并生成规范化内容；
8. 输出 MessagePack 内容包和 SHA-256 content hash；
9. 生成 Rust/TypeScript 开发期索引和审计报告。

运行时只加载验证通过的编译包。开发热重载也必须先通过相同验证，不能绕过 Schema。

## 6. 数据包组合

- 依赖先按拓扑排序；
- 同级包按明确的用户加载顺序，再以 pack ID 作为稳定 tie-breaker；
- 默认禁止两个包静默定义同一 ID；
- 修改已有定义必须使用显式 patch 文件；
- patch 只能修改 Schema 允许的字段；
- 删除内容必须显式声明，并在载入旧存档时给出迁移或缺失内容错误；
- 合并结果和加载顺序进入 content hash。

v1 不支持任意脚本执行。复杂规则由核心提供带版本的声明式组件和效果 ID。

## 7. Patch 格式

v1 使用受限字段操作，不使用依赖数组下标的通用 JSON Patch：

```json
{
  "formatVersion": 1,
  "target": "rfb.monster.dragon.red",
  "set": { "level": 62 },
  "addTags": ["boss-candidate"],
  "removeTags": []
}
```

列表型复杂对象必须带稳定子 ID，patch 按子 ID 增删改，禁止按第几个元素定位。

## 8. Tileset 与本地化

- 内容只提供语义 ID、glyph fallback 和可选视觉标签；
- tileset manifest 把语义 ID 映射到资源；
- 名称和描述只引用 Fluent key；
- 数据包可以附带 locale，但不能覆盖其他包的 key，除非 manifest 显式声明翻译扩展关系；
- 缺失图片 tile 时回退 glyph，缺失当前语言时回退 `en-US`。

## 9. 存档兼容

存档记录：

- 已启用包 ID、版本和 hash；
- 合并后的总 content hash；
- 使用到的定义 ID；
- 必要的迁移 alias 版本。

载入时如果内容集合不同，默认拒绝继续并展示差异。未来可以提供“安全模式”，但不能把缺失定义静默替换成另一对象。

## 10. 安全限制

- 单文件、单包、贴图尺寸和解压后总大小设上限；
- 所有相对路径规范化后必须留在包目录内；
- 禁止远程 URL 在游戏运行时自动下载代码或资源；
- 图片、字体和本地化文件按不可信输入处理；
- 编译器和运行时解析器都进行 fuzz 测试；
- 数据包不能访问文件系统、网络或核心内部对象。

## 11. v1 验收

- 一个基础包可以定义最小地图、玩家、怪物和物品；
- Rust native 与 WASM 加载后产生相同 content hash；
- 重复 ID、悬空引用、循环依赖和非法 patch 都会失败；
- 包加载顺序可复现；
- 缺失本地化和 tileset 映射有明确回退；
- 存档能够验证精确内容集合。
