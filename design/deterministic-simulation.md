# 确定性模拟、随机数与回放规范

状态：P0 规则已确定，参考实现尚未编写

## 1. 原则

相同核心版本、内容哈希、初始存档和命令序列，必须在 Rust 原生与 WASM 中产生相同：

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
  commands: ReplayCommand[];
  checkpoints: ReplayCheckpoint[];
}
```

每条命令记录 `commandSeq`、执行前 revision、回合号和命令 DTO。禁止记录本地化文本代替语义 ID。

检查点默认每 100 个命令生成一次，包含 revision、turn、RNG draw counter 和 state hash。调试版本可以附带规范化快照。

## 6. State hash

- 使用明确版本的规范化序列化结果计算 SHA-256；
- 字段顺序固定；
- map 按 key 排序；
- 不包含日志、时间戳、UI 状态、渲染缓存和本地路径；
- hash Schema 自身有版本号；
- hash 不作为安全签名，只用于一致性和诊断。

## 7. 并发规则

游戏规则在逻辑上单线程串行执行命令。Worker、文件 IO 和资源加载可以并发，但不能并发修改权威世界状态。

后台任务的结果必须通过带序号的消息在确定的同步点提交。完成先后不能改变游戏规则。

## 8. 诊断包

玩家报告不同步或崩溃时，可以选择导出：

- 核心和前端版本；
- 内容 hash；
- 最近的命令回放；
- 最近检查点和 state hash；
- 去除隐私信息的日志；
- 平台和渲染后端信息。

诊断包默认不包含玩家姓名、任意文件路径和完整存档，除非玩家明确选择。

## 9. 验收

- 同一 fixture 在 Windows native、Windows WASM 和 CI Linux native 结果一致；
- 10,000 回合回放不发生 state hash 漂移；
- 保存并重载后继续回放的结果与不中断回放一致；
- 日志等级、语言和渲染后端变化不改变 RNG draw counter；
- 随机数算法或 hash Schema 变化时旧回放给出明确的不兼容错误。
