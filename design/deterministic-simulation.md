# 确定性模拟、随机数与回放规范

状态：P0 规则、RNG、`rfb-replay` v1 和 Tauri 诊断导出已建立

当前 state hash Schema 为 v19：哈希输入覆盖运行时内容包 ID/hash、world ID、当前 `FloorId`、按 ID 排序的离层 `FloorState`、战斗状态、物品实例、怪物携带物、种类/实例知识、秘密 terrain 发现知识、完整任务状态机、持久地牢守护者状态、RNG、世界脉冲和命令序号。contract-v46 因新增权威 `DungeonState` 而显式升级 Schema。

contract-v47 固定 vault 的生成顺序：先绘制规范化基础 terrain/覆盖，再按 group ID、成员位置逐个消费一次深度加权 actor 抽取，最后按 spawn ID 执行既有 loot table 三抽取事务。它没有新增权威状态字段；生成后的 terrain、actor、item、实例分配器、RNG 和 content hash 已进入 Schema v19，因此本切片不升级 state hash Schema。

contract-v48 在房间几何之前先过滤 theme 表：单一候选不抽取，多候选消费一次整数加权抽取；随后按相同规则过滤并选择能放入远端房间的 Vault，无候选时不消费 Vault 抽取并回退为普通房间。房间几何之后，encounter 表按 roll 顺序执行一次怪物权重抽取和一次位置选择；巢穴只执行一次怪物权重抽取，再按实例序号选择多个位置。之后依次生成 Vault encounter group、守护者、怪物携带物、楼层 loot 和 Vault loot。它不增加权威状态字段；v47 已生成楼层不会补套表、Vault 或巢穴，也不会额外消费 RNG，因此 state hash 继续使用 Schema v19。

contract-v49 的 `generationBudget` 算术不消费 RNG。生成器先从 actorSlots 预留巢穴、所选 Vault 群体和仍存活守护者，再以 encounter table rolls 为上限按 ordinal 填充普通遭遇；从 lootPlacements 预留 Vault loot 后，按 ordinal 重复楼层 loot table placement。十层压力地牢的主题表在深度 4 只有一个新主题候选，因此主题分段本身不抽取。v48 已生成楼层和 RNG 状态原样迁移，缺失的新地牢状态只补默认值；state hash 继续使用 Schema v19。

contract-v50 的空间 Vault 管线先按剩余 area/actor/loot 预算过滤候选，再按内容权重抽取；模板变换按规范枚举序、地图原点按行优先枚举，可行原点超过一个时消费一次有界抽取。绘制后的非 wall 矩形使后续重叠候选失效。无可行原点的模板只从候选池移除，不消耗落位槽或预算，然后继续相同流程；候选耗尽即停止。v49 已生成楼层、terrain、实体、物品和 RNG 状态原样迁移，不补绘空间 Vault；state hash 继续使用 Schema v19。

contract-v51 的动态群体阶段按 grouped encounter 权重选择领袖，依次抽取 friends 数量、escort 数量和每个 escort 种类。formation 候选按领袖位置行优先和八方向规范序枚举，多候选只抽取一次；空间不足时不重抽计数，而是先缩 escort、再缩 friends，最小阵容仍失败则原子丢弃该 grouped 候选。群体阶段结束后，剩余 actorSlots 按 plain encounter 规则填充。v50 已生成楼层、实体和 RNG 状态原样迁移，不补生成 friends/escort；state hash 继续使用 Schema v19。

contract-v27 固定程序化楼层的布局、怪物种类/位置、携带物、地面掉落位置和 loot roll 顺序；生成结果已经由 Schema v14 的当前/离层 actor、item、分配器和 RNG 字段覆盖，因此本切片不升级 state hash Schema。

contract-v28 的门开关直接替换权威 terrain ID；contract-v29 的锁定、开锁和破损结果继续使用同一数组。开锁/破门检定固定先抽 percentile，非自动结果再抽 ability contest。contract-v30 的相邻交互列表完全由 terrain、实体和地面物品派生，不消费 RNG。contract-v31 按固定八方向只对尚未发现的隐藏 terrain 执行搜索检定；发现位置作为权威知识进入 Schema v15，普通探索记忆仍不进入 hash。

state hash 与正式存档 DTO 已解耦。Schema v19 使用显式、版本固定的兼容投影，正式 `.rfbsave` 则只保存权威字段；清理存档中的最终攻击、AC、伤害骰和装备派生 modifier 不会静默改变 hash。探索记忆仍保存于每个楼层但不参与 hash，秘密 terrain 知识、任务状态机、当前阶段和守护者击败状态属于权威规则状态并参与 hash。未来规则状态边界变化时必须建立新的 state hash Schema，不得借修改存档序列化顺序隐式更新基准。

## 1. 原则

相同核心版本、内容哈希、初始存档和命令序列，必须在 Windows、Linux、macOS 和 Android 原生 Rust 核心中产生相同：

- RNG 消耗；
- 游戏事件；
- 权威状态；
- state hash；
- 新存档结果。

帧率、动画速度、窗口大小、语言、tileset、日志时间和平台路径不得影响规则结果。

## 2. 新核心 RNG v1

新存档使用版本化 RNG：

- ID：`rfb-rng-xoshiro256ss-v1`；
- 状态：4 个 `u64` 加一个 `u64` draw counter；
- 核心算法：xoshiro256**；
- 单个 64 位 seed 使用 SplitMix64 展开为完整状态；
- 全零状态非法；
- 运算使用 Rust 显式 `wrapping_*` 和固定旋转位数；
- RNG 状态、算法 ID 和 draw counter 必须写入存档和回放检查点。

RNG 不用于密码学、联网身份或安全令牌。

旧版随机数作为独立 `legacy-rng` 兼容模块处理，不能与新 RNG 共用同一个算法 ID。是否要求某个迁移模块完全复刻旧 RNG，由行为基准场景逐项决定。

## 3. 随机数调用规则

- 禁止直接依赖第三方库的默认 RNG；
- 所有规则随机数从显式 `GameRng` 参数取得；
- UI、渲染、粒子和音效使用独立的非权威随机源；
- 遍历 HashMap/HashSet 后随机选择前必须先按稳定 ID 排序；
- 分支不能因为日志、语言或渲染 capability 改变 RNG 调用次数；
- 百分比和权重使用整数拒绝采样，避免浮点舍入差异；
- 测试可以注入脚本 RNG，但正式存档必须记录真实算法 ID。

## 4. 数值确定性

- 权威规则优先使用整数、定点数或有理数；
- 不允许把 `f32`/`f64` 结果用于命中、伤害、AI 决策、掉落和地图生成；
- 溢出行为必须显式：饱和、检查失败或 wrapping，不能依赖编译模式；
- 时间以回合、tick 或整数毫秒表示；
- 排序必须提供完整稳定 tie-breaker；
- Unicode 大小写和区域格式化不能参与规则 ID 比较。

浮点数可以用于渲染插值、音量和非权威动画，但不得写入游戏存档。

## 5. 命令回放 v1

回放文件记录：

```ts
interface ReplayV1 {
  format: "rfb-replay";
  formatVersion: 1;
  coreVersion: string;
  protocolVersion: string;
  contentHash: string;
  initialSaveHash: string;
  rngAlgorithm: string;
  stateHashSchemaVersion: 19;
  commands: ReplayCommand[];
  checkpoints: ReplayCheckpoint[];
}
```

每条命令记录 `commandSeq`、执行前 revision、玩家行动数和命令 DTO。`worldTick`、速度与剩余能量由检查点 state hash 精确覆盖。禁止记录本地化文本代替语义 ID。

检查点默认每 100 个成功命令生成一次，回放结束或导出时还会补充最后一个命令的检查点。检查点包含 revision、turn、RNG draw counter 和 state hash。调试版本可以附带规范化快照。

正式 `.rfbreplay` 文件使用 `RFBREPL\0` magic、容器版本、payload 长度、SHA-256 校验和与 MessagePack payload。开发工具可以读写等价 JSON，但 JSON 不是正式发行载荷。

`ReplayRecorder` 只包装正常的 `Game::dispatch`，不会实现第二套规则路径。它支持：

- 自动构造命令序号的记录入口；
- 记录已有 `GameCommandEnvelope`；
- 不结束游戏会话即可导出回放快照；
- 从任意新游戏或载入后的存档状态开始新的回放段；
- 播放前检查核心版本、协议、内容 hash、RNG 和初始状态 hash；
- 播放时检查命令上下文、检查点调度和所有检查点内容。

Tauri 原生会话持有 `ReplayRecorder`，前端可以导出正式 `.rfbreplay` 文件。新游戏和每次载入存档都会开始新的回放段；只有核心成功接受的命令会被记录。回放文件不嵌入完整初始存档，载入存档后的回放需要配合具有相同 `initialSaveHash` 的初始状态复验。

## 6. State hash

- 使用明确版本的规范化序列化结果计算 SHA-256；
- 字段顺序固定；
- map 按 key 排序；
- 不包含日志、时间戳、UI 状态、渲染缓存和本地路径；
- hash Schema 自身有版本号；
- hash 不作为安全签名，只用于一致性和诊断。

差分测试使用独立的快照规范化 Schema v1：去除时间戳、会话 ID、本地路径、平台窗口信息，稳定排序语义无序集合，保留事件顺序，并拒绝权威浮点值。该规范化 hash 用于 fixture/差分诊断，不替代核心 `state_hash()`。

## 7. 并发规则

游戏规则在逻辑上单线程串行执行命令。Tauri 异步 command、文件 IO 和资源加载可以并发，但不能并发修改权威世界状态。

后台任务的结果必须通过带序号的消息在确定的同步点提交。完成先后不能改变游戏规则。

## 8. 诊断包

崩溃诊断由桌面端自动写入本机私有目录，不依赖玩家在崩溃后主动导出。当前 v1 包含：

- 应用和核心协议版本；
- 内容 hash；
- 去除隐私信息的日志；
- 平台和渲染后端信息。

诊断包不包含玩家姓名、任意文件路径、完整存档或玩家文本，也不会自动上传。最近命令回放、检查点和 state hash 在确认隐私与大小边界后再加入后续格式版本。生命周期和轮换规则见[桌面崩溃诊断闭环 v1](crash-diagnostics-v1.md)。

## 9. 验收

- 同一 fixture 在 Windows、CI Linux、macOS 和 Android ARM64 原生核心结果一致；
- 10,000 回合回放不发生 state hash 漂移；
- 保存并重载后继续回放的结果与不中断回放一致；
- 日志等级、语言和渲染后端变化不改变 RNG draw counter；
- 随机数算法或 hash Schema 变化时旧回放给出明确的不兼容错误。

当前自动测试已经覆盖 10,000 回合无漂移、每 100 命令检查点、最终检查点、RNG draw counter、存档重载续播、命令和上下文篡改、错误初始状态、二进制/JSON 回环、checksum 损坏检测，以及 Tauri 导出复验、失败命令排除和载入后新回放段。
