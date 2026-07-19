# 确定性模拟、随机数与回放规范

状态：P0 规则、RNG、`rfb-replay` v1 和 Tauri 诊断导出已建立

当前 state hash Schema 为 v9：哈希输入覆盖运行时内容包 ID/hash、world ID、玩家与怪物战斗状态、基础速度、剩余行动能量、状态、抗性、物品、RNG、玩家行动数、世界脉冲和命令序号。active contract-v11 只为已经发生的伤害与死亡事件增加结构化协议 outcome，没有新增权威状态，因此不虚增 state hash Schema。Schema v1-v8 只作为历史基准保留。

state hash 与正式存档 DTO 已解耦。Schema v9 使用显式、版本固定的兼容投影，正式 `.rfbsave` 则只保存权威字段；清理存档中的最终攻击、AC、伤害骰和装备派生 modifier 不会静默改变 v9 hash。未来规则状态边界变化时必须建立新的 state hash Schema，不得借修改存档序列化顺序隐式更新基准。

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
  stateHashSchemaVersion: 9;
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
