# Contract v290：捕获球与骑兵闭环

状态：已实现。Protocol `1.192`，State Hash Schema `v96`，内容包 `1.303.0`，
save v1，active baseline `contract-v290`。

## 1. 权威来源与内容身份

- 固定来源为 RFB `master:lib/edit/k_info.txt` source index 704、`master:src/cmd6.c`、
  `master:src/gf.c` 与 `master:src/dungeon.c`，均通过 Git 对象读取。
- 新增且由 class 方向拥有的正式物品 ID 只有 `demo.item.capture-ball`；没有新增 ability、
  material、affix 或 actor ID。
- 原版 `W:15:0:0:120:1000` 表示等级 15、12.0 磅、价值 1000；先前计划中的 2.0 磅是
  笔误。本地保留不可堆叠、副手 shield 槽、`A:15/4` 基础掉落和两座杂货店 25% 维护资格。
- 中文名严格使用 RFB `master` 的“捕获球”。

## 2. 捕获与释放

- 空球使用既有 `UseItem` + entity 目标；满球使用既有 `UseItem` + direction 目标。捕获
  当前坐骑时，目标可与玩家同格，不增加新命令或待处理输入类型。
- `capturePolicy` 由原版怪物旗标导入为 `normal | pet-only | immune`。`UNIQUE2`、questor
  和原版三个特殊合体怪免疫；Unique/Nazgûl 只允许已经成为宠物时捕获。
- 宠物生命阈值为 `4 * maxHp`，普通野怪必须低于 `3/20 * maxHp`；通过资格后才执行
  `hp <= randint0(threshold)`，资格和生命预检失败不推进 RNG。
- 捕获当前坐骑必然解除骑乘并清除羁绊；浮空免除坠落伤害，否则按怪物等级 `+3` 结算。
- 球内只保存 kind、速度、当前/最大生命和经验。正常释放生成新的宠物实体，不恢复旧实体
  ID、临时状态、召唤或群体关系。
- 正常激活只能释放到所选相邻合法格，失败保持球内状态。丢弃或投掷在附近强制释放并有
  `1/4` 敌对概率；显式摧毁、弹道/怪物/地震摧毁和 Mogaminator 摧毁均在附近强制按宠物
  阵营释放。

## 3. 生命周期与唯一怪物

- 背包或装备中的普通捕获怪每 30 tick 恢复 `maxHp / 100`；商为零时以 `1/2` 概率恢复 1，
  原版再生标记使恢复量翻倍。Unique/Nazgûl 使用 600 tick 周期。
- Unique/Nazgûl 在 active floor、stored floor、商店、家或任意权威物品实例的捕获球中时，
  都从随机生成候选排除；不保存第二份计数状态。
- 物品名称与详情投影球内怪物名、生命和经验；装备面板提供激活入口。

## 4. 存档、协议与验证

- 四种物品 save DTO 的 `capturedActor` 是必填可空字段。载入严格拒绝未知/免疫 actor、
  非捕获球内容物、零速度、非正生命、生命超过上限等非法状态；不提供旧开发存档兼容。
- 捕获状态进入 State Hash Schema v96。商店低概率库存增加共享初始化 RNG，因此 26 条
  active exact fixture 全量刷新并复验，零 waiver。
- 聚焦验证覆盖资格短路与 RNG、Unique 宠物、当前坐骑落马和羁绊重置、新实体释放、阻塞
  释放、精确敌对骰、恢复周期、投影、save/state-hash 往返及同格前端目标选择。

内容 hash：`538cce0f525d1530dbb109f4cf75074c69130b09eebca10d672628ad770467e5`。
