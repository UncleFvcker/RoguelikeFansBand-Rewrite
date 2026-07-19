# RFB 新存档格式 v1

状态：v1 容器、独立权威存档 DTO、校验读写和 Windows/Tauri 原生目录事务已实现；跨 schema 迁移链与旧 C 存档完整导入仍未实现

## 1. 基本决定

- 扩展名：`.rfbsave`；
- 容器格式：RFB 自描述二进制容器；
- 权威载荷：MessagePack；
- 调试导出：规范化 JSON，但不能作为正式可继续游戏的默认格式；
- 校验：SHA-256；
- 默认不压缩 v1 载荷，后续通过容器 flags 增加版本化压缩；
- 旧 C 存档只通过独立导入器读取，新核心不再写旧格式。

## 2. 容器布局

```text
magic              8 bytes   "RFBSAVE\0"
container_version  u16 LE     1
flags              u16 LE
header_length      u32 LE
payload_length     u64 LE
payload_sha256     32 bytes
header_json        header_length bytes UTF-8
payload_msgpack    payload_length bytes
```

读取器必须先验证长度上限，再分配内存。未知 flags、超大长度、截断文件和 hash 不符必须给出明确错误，不能 panic。

## 3. Header

Header 只含无需解码完整世界即可展示的信息：

```ts
interface SaveHeaderV1 {
  format: "rfb-save";
  saveSchemaVersion: 1;
  gameVersion: string;
  protocolVersion: string;
  slotName: string;
  createdAt: string;
  savedAt: string;
  characterSummary: {
    displayName: string;
    level: number;
    locationKey: string;
    turn: number;
  };
  contentId: string;
  contentHash: string;
  payloadEncoding: "messagepack";
}
```

Header 不可信，显示前需要长度限制和转义；载入是否成功以 payload 验证和迁移结果为准。

`slotName` 是桌面原生槽使用的可选显示元数据。Rust 反序列化对缺失字段使用空字符串默认值，因此本字段的加入不破坏已经生成的 v1 存档；手动导出的存档当前写入空名称。桌面目录事务和恢复行为见 [桌面原生存档与诊断 v1](desktop-native-storage-v1.md)。

## 4. Payload

```ts
interface SavePayloadV1 {
  schemaVersion: 1;
  revision: number;
  turn: number;
  worldTick: number;
  lastCommandSeq: number;
  terrain: TerrainSaveDto;
  player: PlayerSaveDto;
  entities: ActorSaveDto[];
  items: ItemSaveDto[];
  inventory: InventoryItemSaveDto[];
  equipment: EquipmentItemSaveDto[];
  nextItemInstanceSerial: number;
  explored: boolean[];
  rng: RngSaveDto;
  contentId: string;
  contentHash: string;
  worldId: string;
}
```

当前桌面垂直切片已经把地面 `items`、`inventory` 物品堆、`equipment` 装备列表和 `nextItemInstanceSerial` 写入 payload。存档使用独立的 `PlayerSaveDto`、`ActorSaveDto` 和物品存档 DTO，不再复用面向前端的 `PlayerDto`、`EntityDto`、`InventoryItemDto` 或 `EquipmentItemDto`。

Rust 运行时内部只保留一个 `ItemInstance` 集合，`ItemLocation` 明确区分 `Ground(position)`、`Inventory` 和 `Equipped(slotId)`。拾取、整堆丢弃、装备与卸下只改变同一实例的位置；部分拆堆才分配新的稳定实例 ID。v1 存档线格式继续投影为 `items`、`inventory` 和 `equipment` 三个列表，以兼容现有文件和 contract-v9，但这三个列表不再对应三套核心结构体。

玩家存档保存实例 ID、种类 ID、位置、当前生命、自然最大生命、基础速度、当前 `energyNeed`、状态列表与抗性 profile；怪物保存相同的权威运行状态。状态保存稳定 kind ID、强度、剩余 tick 和可选来源 ID；普通抗性不显式写入稀疏列表。最终速度、攻击、防御、近战能力、AC、伤害骰、装备 modifier、死亡标志、glyph 和本地化文本均不写入新存档，而是在载入后重新派生。旧 v1 存档缺失状态/抗性字段时按空集合迁移。

背包与装备项保存稳定实例 ID、内容 kind ID、数量及装备槽 ID，不保存选择复选框或 HTML 面板状态。载入后必须验证内容引用、实例 ID 唯一性、`maxStack`、槽位匹配、槽位唯一性，以及生成实例序号不能落后于任何 `generated.item.N`。旧存档缺失 `equipment` 时按空列表载入，缺失分配序号时从所有现有实例 ID 推导。玩家负生命值代表已死亡，可安全保存和重载；`isDead` 仅是协议派生字段。

协议 1.3 增加 `explored` 布尔数组保存 Rust 权威地图记忆。旧存档缺失该字段时按空记忆载入并揭示玩家当前 FOV；该数组不参与 state hash，因此地图探索显示不会改变规则基准或回放检查点。当前 state hash Schema v9 使用独立的历史兼容投影，不再直接哈希当前存档 DTO；状态与抗性属于权威规则字段并明确进入 v9 hash。

禁止保存：

- Rust 内存布局和枚举下标；
- TypeScript UI 状态；
- RenderWorld、纹理和动画；
- 已本地化完成的系统句子；
- 临时计算缓存；
- 绝对文件路径；
- 网络令牌和崩溃报告信息。

允许保存玩家自定义名称和模组声明的用户内容，但必须限制长度。

## 5. 写入事务

桌面版执行：

1. 在同一目录创建唯一临时文件；
2. 完整写入并 flush；
3. 重新读取 header 和 checksum 做快速验证；
4. 尽平台能力执行 `fsync`；
5. 将现有正式存档轮换为 `.bak1`；
6. 原子 rename 临时文件为正式存档；
7. 保留最近 3 个备份；当前数量固定，未来再提供设置；
8. 失败时保留最后一个有效正式存档。

不得先删除旧存档再写新文件。临时文件清理由启动时的恢复流程处理。

Android 使用应用私有目录和同样的临时文件、校验、原子替换与备份流程；通过系统文件选择器进行玩家主动导入、导出和分享。各平台路径由 Tauri 适配层提供，核心存档格式不感知操作系统。

## 6. 载入与恢复

载入顺序：

1. 验证 magic、容器版本、flags 和长度；
2. 验证 payload SHA-256；
3. 解析 Header 和 MessagePack；
4. 验证 Schema 与数值上限；
5. 验证内容包集合；
6. 连续执行迁移；
7. 构建临时世界并运行不变量检查；
8. 全部成功后替换当前会话。

正式文件损坏时，按 `.bak1` → `.bak2` → `.bak3` 查找最近有效备份，并在恢复前告知玩家。损坏文件不得静默覆盖。

## 7. 迁移规则

- 迁移是 `v1 → v2 → v3` 连续函数；
- 每一步输入输出都有 fixture 和 hash；
- 增加字段必须提供默认值或可推导规则；
- ID 改名通过显式 alias 表；
- 无法无损迁移时停止并说明具体缺失内容；
- 迁移在内存中的临时副本上执行；
- 成功载入旧版本不会立刻覆盖原文件，下一次保存才写新版本；
- 发布版本不能删除仍在支持窗口内的迁移器。

## 8. 旧 C 存档导入

旧格式读取器作为隔离工具存在：

```text
crates/rfb-legacy-import/
```

导入流程输出转换报告，包括：

- 旧版本识别结果；
- 已转换字段；
- 无法转换或采用默认值的字段；
- 名称到稳定 ID 的映射；
- 内容包要求；
- 新存档 hash。

导入器只读旧文件，绝不原地覆盖。旧存档解析器必须限制字符串长度、计数和分配大小，并使用 fuzz/corpus 测试。

当前第一阶段已经实现链式 XOR 解码和 409 字节稳定前缀解析，覆盖版本、保存元数据、63 项 RNG 状态和选项位；三份本地样本通过长度、SHA-256、版本和字段级精确复验。旧 `player_type`、物品、地图等可变布局尚未进入解析范围，也不会直接映射为新核心结构。

## 9. 内容包和模组

存档记录每个包的 ID、版本、hash 和加载顺序。载入时分为：

- 完全匹配：正常载入；
- 版本不同但存在内容迁移器：迁移后载入；
- 缺失或 hash 不符：默认拒绝，展示差异；
- 用户明确进入未来的恢复模式：只在复制文件上操作，并生成不可逆警告。

## 10. 安全与隐私

- 文件大小、地图数量、实体数量、字符串和嵌套深度均设上限；
- 不解析存档内的脚本、HTML 或外部路径；
- MessagePack 未知扩展类型默认拒绝；
- 导入文件不能触发网络请求；
- Header 中的玩家文本按不可信内容转义；
- 崩溃报告上传存档必须由玩家单独确认。

## 11. v1 验收

- Windows、Linux、macOS 和 Android 原生核心读写相同 fixture；
- 保存 → 读取 → 保存得到语义相同状态和相同 state hash；
- 模拟断电不会丢失最后一个有效备份；
- 单字节损坏能被 checksum 发现；
- 截断、超大长度和畸形 MessagePack 不会 panic；
- v1 → v2 示例迁移证明连续迁移机制可用；
- 三个仅保存在本机 `.local/` 中的 `v1.3.0.7` 旧存档样本可以导入或给出结构化失败报告。
