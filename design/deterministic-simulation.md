# 确定性模拟、随机数与回放规范

状态：P0 规则、RNG、`rfb-replay` v1 和 Tauri 诊断导出已建立

当前 state hash Schema 为 v40：哈希输入覆盖运行时内容包 ID/hash、world ID、当前 `FloorId`、当前 dungeon instance ID、当前与离层的连接 ID→位置→解析目标、离层 floor 实例身份、区域 ID/theme/局部表引用/格集合、actor 的 pack identity/behavior/alerted、summon identity/lifetime、怪物施法剩余冷却与已观察玩家抗性、战斗状态、物品实例、怪物携带物、种类/实例知识、秘密 terrain 发现知识、含重接次数的完整任务状态机、持久地牢守护者、入口守卫与实例序号/retained 状态、campaign 胜利/退休/最终分数、玩家 Race/Class/Personality/build 身份、技能聚合与成长、角色成长 progress、资源池、已学能力、能力熟练度/统计/冷却（含先天技法）、全局召唤指令/Guard 锚点、RNG、世界脉冲和命令序号。contract-v83 因持久/瞬时侦测知识与 FOV 真值隔离规则升级 Schema v36；contract-v86 因怪物施法剩余冷却升级 Schema v37；contract-v88 因 smart caster 抗性记忆升级 Schema v38；contract-v89 因持久召唤指令升级 Schema v39；contract-v90 因技法资源池与先天能力熟练度扩展资源/能力状态域升级 Schema v40。

contract-v47 固定 vault 的生成顺序：先绘制规范化基础 terrain/覆盖，再按 group ID、成员位置逐个消费一次深度加权 actor 抽取，最后按 spawn ID 执行既有 loot table 三抽取事务。它没有新增权威状态字段；生成后的 terrain、actor、item、实例分配器、RNG 和 content hash 已进入 Schema v19，因此本切片不升级 state hash Schema。

contract-v48 在房间几何之前先过滤 theme 表：单一候选不抽取，多候选消费一次整数加权抽取；随后按相同规则过滤并选择能放入远端房间的 Vault，无候选时不消费 Vault 抽取并回退为普通房间。房间几何之后，encounter 表按 roll 顺序执行一次怪物权重抽取和一次位置选择；巢穴只执行一次怪物权重抽取，再按实例序号选择多个位置。之后依次生成 Vault encounter group、守护者、怪物携带物、楼层 loot 和 Vault loot。它不增加权威状态字段；v47 已生成楼层不会补套表、Vault 或巢穴，也不会额外消费 RNG，因此 state hash 继续使用 Schema v19。

contract-v49 的 `generationBudget` 算术不消费 RNG。生成器先从 actorSlots 预留巢穴、所选 Vault 群体和仍存活守护者，再以 encounter table rolls 为上限按 ordinal 填充普通遭遇；从 lootPlacements 预留 Vault loot 后，按 ordinal 重复楼层 loot table placement。十层压力地牢的主题表在深度 4 只有一个新主题候选，因此主题分段本身不抽取。v48 已生成楼层和 RNG 状态原样迁移，缺失的新地牢状态只补默认值；state hash 继续使用 Schema v19。

contract-v50 的空间 Vault 管线先按剩余 area/actor/loot 预算过滤候选，再按内容权重抽取；模板变换按规范枚举序、地图原点按行优先枚举，可行原点超过一个时消费一次有界抽取。绘制后的非 wall 矩形使后续重叠候选失效。无可行原点的模板只从候选池移除，不消耗落位槽或预算，然后继续相同流程；候选耗尽即停止。v49 已生成楼层、terrain、实体、物品和 RNG 状态原样迁移，不补绘空间 Vault；state hash 继续使用 Schema v19。

contract-v51 的动态群体阶段按 grouped encounter 权重选择领袖，依次抽取 friends 数量、escort 数量和每个 escort 种类。formation 候选按领袖位置行优先和八方向规范序枚举，多候选只抽取一次；空间不足时不重抽计数，而是先缩 escort、再缩 friends，最小阵容仍失败则原子丢弃该 grouped 候选。群体阶段结束后，剩余 actorSlots 按 plain encounter 规则填充。v50 已生成楼层、实体和 RNG 状态原样迁移，不补生成 friends/escort；state hash 继续使用 Schema v19。

contract-v52 在 Vault 绘制后、actor/loot 生成前执行特殊地形阶段。每个槽先按深度候选权重抽取 terrain 条目，再从按地图行优先规范化的 room 或 corridor 位置中抽取；单条目或单位置不消费 RNG。无位置候选只从当前槽移除并继续回退，成功位置立即改写 terrain 并进入后续占位集合。v51 已生成楼层及 RNG 状态原样迁移，不补放 trap/door/rubble；state hash 继续使用 Schema v19。

contract-v53 在房间阶段前生成 cavern 基础地貌：从中心开始，每步将四向连通前沿按 `y/x` 排序，多候选消费一次位置抽取，直到精确达到面积预算。房间阶段先按规范排序的形状权重选种，再从位置/尺寸候选中抽取；每个槽为未来房间保留最小面积，保证精确房间数量而不超总面积。房间按稳定槽序连接，encounter/loot 按 ordinal 轮转到非入口房间。v52 已生成楼层及 RNG 状态原样迁移，不补绘 cavern 或重建房间；state hash 继续使用 Schema v19。

contract-v54 的 lake 使用同一规范四向前沿，插入序列固定深水核心与浅水外缘。river 依次抽取边界、边界坐标和双轴推进方向，再从完整中心线的规范前沿扩展浅水岸。房间和 tunnel 后续覆盖水文以保证落点与主链连通；面积是阶段绘制预算。v53 已生成楼层及 RNG 原样迁移，不补绘 lake/river；state hash 继续使用 Schema v19。

contract-v55 的 maze 将格点按行优先编号，从抽取根节点开始，以固定 `north/east/south/west` 候选执行随机 DFS；destroyed 从规范中央候选抽取震中，再沿 `y/x` 四向前沿扩展；streamer 依次抽取权重、普通墙起点、八向方向和规范候选，空间不足时只从剩余 wall 回退。contract-v56 的 pit 依次抽取固定数量 roster，按 actor level 降序和 ID 升序排序，再按内室行优先坐标及归一化中心距离映射阶位；复合房间 footprint 先于普通 encounter/loot 保留。contract-v57 的 maze-only 在 DFS 后不生成 rooms/corridors；两次规范 BFS 产生入口与远端锚点，同距离按 `y/x` 决胜，固定陷阱取锚点路径中点，actor/loot 候选按距入口降序和 `y/x` 升序落位。v56 已生成楼层及 RNG 原样迁移，不重建 maze-only；state hash 继续使用 Schema v19。

contract-v58 先在旧入口与旧下楼位置绘制第一组主 up/down，使旧主路径保持稳定；Vault 完成后，再按连接 ID 顺序从行优先合法 floor 候选中对附加楼梯和 shaft 各执行一次有界抽取。连接格进入后续 feature、actor、loot 占位集合。目标楼层按 `targetConnectionId` 独立落位。v57 已生成楼层的连接列表为空时不重建地图或推进 RNG，继续使用 terrain 标签回退；新连接状态进入 state hash Schema v20。

contract-v59 按楼层内生成群体的稳定顺序分配 `{floorId}.pack.N`，并把角色和行为冻结到 actor 状态。AI 按 actor ID、固定八方向和共享目标格预留依次决策；`surround` 对玩家相邻格的分配和 `guard-leader` 跟随都不消费 RNG。leader 死亡时按稳定 actor 顺序清除同 pack 成员身份。v58 存档缺失 pack 字段时不重建群体、不推进 RNG，旧 actor 回退独立 `seek`；pack 状态进入 Schema v21。

contract-v60 先按 region ID 规范候选并执行整数权重无放回抽样，再沿房间序列建立均匀锚点；非锚点房间按中心 Manhattan 距离与区域顺序决胜。actor/loot 预算按区域顺序整除并分配余数，位置只在所属房间内抽取。区域状态按 region ID、格坐标排序后进入哈希。v59 存档缺失区域时保留空列表、不补生成、不推进 RNG；区域状态进入 Schema v22。

contract-v61 只在 paused 任务成功重入时递增 `retakesUsed`。`preserve-floor` 直接恢复已保存楼层且不抽 RNG；`regenerate-floor` 先按 task ID 丢弃全部已保存成员层，再沿普通楼层生成顺序使用当前 RNG，并把计数目标限制为 `required - current`。次数耗尽和无效地表放弃都不抽 RNG。v60 存档缺失计数时按 0 载入；该计数进入 Schema v23。

contract-v62 在确定性生成中先按既有顺序完成区域选择、特殊 footprint 宿主归属和固定内容占位，再联合分配区域普通 actor/loot。区域动态群体使用稳定 region ID 前缀；同一 region 的怪物寻路只接受 region cells 内的路径。Vault/pit/feature/guardian 的组合不增加额外迁移抽取；v61 及更早生成楼层不会回补区域组合。

contract-v63 的显式连接树完全由内容定义决定，不增加生成抽取。多个最终叶层各自按既有 guardian 阶段生成稳定镜像；第一次镜像死亡将共享 `guardianDefeated` 从 false 改为 true，并按楼层 ID 和镜像 ID 的稳定集合删除其他离层镜像及其携带物，不触发额外掉落或事件。已征服后的未访问叶层跳过 guardian 生成且不消费额外 RNG；v62 存档连接集合若与新树不兼容，只清除连接索引并回退 terrain 标签，不改动地图、实体、物品或 RNG。

contract-v64 先按 transform、origin 和规范入口顺序枚举 Vault 候选。每个入口 connector 使用固定 `north/east/south/west` BFS，最多雕刻 12 格；已有 connector 可作为后续入口目标。模板与 connector 先画入临时 terrain，只有全部潜在可通行格形成单一四向分量时才提交。连接路径不抽 RNG；候选加权、落位下标和失败后的后续候选仍沿用 v50 顺序。v63 已生成楼层只读取已保存 terrain，不补 connector、不重放候选，也不推进 RNG。

contract-v65 在从地表进入 dungeon 时只消费实例序号状态，不消费生成 RNG：每座 dungeon 的 `nextInstanceOrdinal` 加一并组成 `<dungeonId>.instance.<ordinal>`。同实例连接使用当前 ID 查找离层 floor；返回地表按实例 ID过滤清理。contract-v66 在楼层生成末尾消费动态连接候选 RNG，写入连接目标并在到达时修正返回连接。contract-v67 的硬进入条件在实例序号与 RNG 消耗前检查；入口守卫使用不抽 RNG 的 `GuardPosition` 决策，击败状态进入 Schema v26。v66 旧存档缺字段时抑制新增入口守卫，不回填实体或推进 RNG。普通 dungeon 返回地表仍清理整个实例，下一次进入重新生成；相同 seed、存档和命令序列仍产生相同实例树与 hash。

contract-v68 在守护者/任务事件结算后按稳定 dungeon/task 状态计算 campaign 计数，不消费 RNG。评分只使用整数饱和算术和回合整除；成功或失败的 `Retire` 都不运行怪物调度，成功时保存最终分数，退休后的命令被拒绝。v67 旧存档缺 campaign 状态时按持久 dungeon 征服状态推导 active/victorious，不补生成内容或推进 RNG。campaign 状态进入 Schema v27。

contract-v69 在返回地表时仅按内容声明决定清理或保留实例；续接 retained 实例不消费生成 RNG，TTL 淘汰只在下一次进入前发生并按稳定实例序号分配新实例。retained ID/回合进入 Schema v28，旧存档缺字段按 reset 默认迁移。实例级属性知识只随具体物品实例是否仍存在而保留或清理，种类级知识不受楼层淘汰影响。

contract-v70 的 HP 成长序列在角色出生时使用独立于世界模拟的 seed 派生 RNG 一次生成并随存档保存；经验增加只改变 progress，不改变地牢生成 RNG，等级提升按固定 RFB 阈值顺序结算。未胜利时达到 50 级后继续累积经验但不越过等级上限；campaign 胜利事件和胜利/退休载入都以零经验增量重新运行同一结算，确定性释放封顶经验到 100 级。属性点命令不推进世界脉冲，18/xx 桶和自然/有效属性投影均由已保存 progress 与装备 modifier 派生。v71 的构筑选择不消费模拟 RNG；技能 base/growth 按等级从内容集合聚合，出生装备使用稳定生成实例 ID，物品种类/实例知识随存档校验。v72 的四类技能检定使用正式模拟 RNG，并在事件中保存已结算的百分位/对抗骰；无候选 perception、范围或视线外 stealth 不抽骰，装置失败不消费物品，旧存档缺少 alerted 时只按内容默认值恢复。v73 的能力学习不抽 RNG；施法只有在职业、学习、书本和资源前置检查通过后才扣资源并抽一次失败率骰，失败仍保留已扣资源，成功后才解析目标并按既有伤害/掉落顺序继续抽取。资源不足拒绝和缺字段存档迁移都不推进 RNG。v74 的恢复本身不抽 RNG：`Wait` 在既有调度结束后应用等待恢复；`Rest` 每步先真实运行同一调度器，再按死亡、受伤、可见敌人、恢复、满资源和回合上限的固定顺序结算。一次 Rest 只记录一个命令/revision，但 `turn` 与 `worldTick` 反映实际执行回合；自身治疗仍只在施法前置通过后消费既有失败率骰。v75 的能力进度更新不额外抽 RNG：成功/失败按内容增量截断，Mana 成本和失败率由持久熟练度纯函数派生；冷却只按实际完成世界回合递减，冷却/资源/前置拒绝发生在失败率骰之前。上述边界进入 Schema v34。

contract-v27 固定程序化楼层的布局、怪物种类/位置、携带物、地面掉落位置和 loot roll 顺序；生成结果已经由 Schema v14 的当前/离层 actor、item、分配器和 RNG 字段覆盖，因此本切片不升级 state hash Schema。

contract-v28 的门开关直接替换权威 terrain ID；contract-v29 的锁定、开锁和破损结果继续使用同一数组。开锁/破门检定固定先抽 percentile，非自动结果再抽 ability contest。contract-v30 的相邻交互列表完全由 terrain、实体和地面物品派生，不消费 RNG。contract-v31 按固定八方向只对尚未发现的隐藏 terrain 执行搜索检定；发现位置作为权威知识进入 Schema v15，普通探索记忆仍不进入 hash。

state hash 与正式存档 DTO 已解耦。Schema v40 使用显式、版本固定的兼容投影，正式 `.rfbsave` 则只保存权威字段；清理存档中的最终攻击、AC、伤害骰、装备派生 modifier、能力失败率、恢复速率和可用性标志不会静默改变 hash。探索记忆仍保存于每个楼层但不参与 hash，秘密 terrain 知识（包括持久侦测）、任务状态机、最终守护者与入口守卫击败状态、campaign 状态、dungeon instance 身份、连接/区域、pack/召唤身份、怪物施法剩余冷却和已观察抗性、角色构筑与成长、资源、能力进度、召唤指令和 actor statuses 属于权威规则状态并参与 hash。怪物的施法频率、基础权重和战术阈值由已哈希内容包固定；HP/状态/距离、敌我计数和已观察抗性只从当前权威状态纯计算。召唤目标/移动选择不抽 RNG，近战照常抽命中与伤害；指令切换不运行任何世界调度。未来规则状态边界变化时必须建立新的 state hash Schema，不得借修改存档序列化顺序隐式更新基准。

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
  stateHashSchemaVersion: 35;
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

v76 的学习容量由 Class 内容与角色 progress 纯函数派生；容量满学习、重复遗忘和其他能力前置拒绝均发生在任何能力 RNG 前。遗忘只移除已学能力集合并保留能力进度，重新学习不抽额外 RNG；旧存档缺少能力进度时仍按当前内容初值迁移。学习容量属于内容锁定的派生投影，因此沿用 state hash Schema v34。

v77 的范围能力先验证目标路径，再扣资源、抽施法失败率和记录熟练度；无效目标不推进 RNG。有效爆发只抽一次基础伤害骰，随后按 RFB `distance()` 的稳定距离层和 `(baseDamage + distance) / (distance + 1)` 整数衰减复用每个 actor 的抗性/死亡/掉落顺序。定点目标不因中途 actor 停止，方向目标在首个 actor 停止；墙体截断传播，footprint 按距离、`y`、`x` 排序。零目标仍消耗资源并保留一次伤害骰，范围半径来自内容而非存档，因此 state hash 继续使用 Schema v34。

v78 的方向射线先验证八向路径，再扣资源、抽施法失败率和记录熟练度；actor 不阻挡，墙体/边界截断，路径按近到远稳定结算，每次施法只抽一次基础伤害骰并复用到所有命中 actor。空射仍是成功施法并消费资源/伤害骰，方向以外模式在任何能力 RNG 前拒绝；射线 footprint 只存在于事件和回放，state hash 继续使用 Schema v34。

v79 的固定八向锥形先验证方向目标，再按中心线逐层展开；actor 不阻挡，墙体/边界截断，候选格按近到远、横向距离和坐标稳定排序，侧向目标使用既有整数距离衰减并共享一次基础伤害骰。空锥仍消费资源并投一次伤害骰，非方向模式在任何能力 RNG 前拒绝；锥形 footprint 不写入存档，state hash 继续使用 Schema v34。

v80 的定点/实体射线在目标存在、可见且不超距后才扣资源并抽施法失败率；核心用同一整数 Bresenham 误差从玩家推进到目标，并沿相同斜率继续到内容射程上限。actor 不阻挡，墙体/不可行走地形/边界截断，路径按近到远稳定结算并共享一次基础伤害骰。自身、缺失、不可见或超距目标在 Mana/RNG/熟练度前拒绝；延长 footprint、目标顺序与事件只存在于命令/回放，save 与 state hash 仍使用 Schema v34。

v81 的 `teleport` 先验证 position 落点：目标必须非当前格、在地图内、处于 Chebyshev 射程内、当前可见、满足 line of effect、可行走且无存活 actor 占据。验证失败不扣 Mana、不抽施法或能力 RNG、不改变能力进度；成功才按既有能力失败率、资源、熟练度和冷却规则结算。成功传送不增加随机骰，位置更新随后复用普通移动的被动感知、陷阱和死亡处理；落点与事件不写入 save，state hash 继续使用 Schema v34。

v82 的 `summon` 在扣 Mana和抽施法失败率之前收集全部落位。候选格按 Chebyshev 距离、`y`、`x` 稳定排序，必须可行走且不被玩家、actor 或地面物品占用；不足配置数量时整次拒绝，不改变资源、RNG 或能力进度。空间足够时只抽既有一次失败率骰；失败仍支付 Mana，成功不再抽额外位置 RNG，而是按稳定顺序生成 `summon.<ability-id>.<command-seq>.<ordinal>`。召唤生命周期在每个玩家完成的世界回合末按实体 ID 顺序递减，到期按同一顺序发出移除事件。owner/source/lifetime 写入 save 并进入 state hash Schema v35。

v83 的 `detect` 只接受 `self` 目标；非法目标和资源不足在任何能力 RNG 前拒绝。合法施法按既有规则支付 Mana并抽一次失败率骰；失败不产生侦测结果，成功后按当前地图、Chebyshev 半径、FOV/line of sight、尚未发现、存在隐藏投影和 terrain tag 类别依次过滤，再按距离、`y`、`x` 稳定排序。空结果仍是成功施法且不额外抽 RNG。持久结果写入 `revealedTerrain` 并进入 save/state hash Schema v36，瞬时结果只进入 `ability.detect` 事件且不改变普通地图知识。

v84 的 `transform-terrain` 只接受 position 中心；中心必须在地图、内容射程、FOV 和 line of effect 内，非法目标和资源不足在任何能力 RNG 前拒绝。有效中心在支付 Mana 前按 RFB 距离、FOV、中心 line of effect、规范化来源 terrain 集、占用格、地图边界、floor connection 和连接 terrain tag 收集候选，并按距离、`y`、`x` 稳定排序。失败施法支付 Mana但不写 terrain；成功后不抽额外 RNG，一次提交预收集集合并移除对应 `revealedTerrain`，空集合仍返回 `ability.terrain-transform`。实际格通过 `changedCells` 更新并由既有当前/离层 terrain 数组进入 save/hash；state hash 保持 Schema v36，运行时不做自动连通修复。

v85 的 `sequence` 在旧单效果读取路径之上声明 2–8 个有序 actor 效果。目标前置、资源、整次失败率、熟练度和冷却只结算一次；成功后子效果严格按数组索引执行。伤害骰只在轮到该效果且目标仍存在时抽取；无目标时全部记录 `no-target`，前序击杀后续记录 `target-dead`，均不抽被跳过效果的 RNG。状态抗性以固定整数比例把 vulnerable/normal/resistant/strong 缩放为 150/100/50/35%，immune 为 0；非免疫最低 1 tick。部分无效不回滚前序效果，重复状态复用既有稳定堆叠规则。状态原本已进入 save/hash，因此 state hash 保持 Schema v36。

v86 的怪物施法先为每个已警戒、未冷却 caster 固定抽一次 1–100 频率骰，再纯计算过滤超距、墙体和路径友军；频率通过且有候选时才按声明顺序总权重抽一次选择骰，随后按效果顺序抽伤害 RNG。频率失败或无候选不抽权重/效果 RNG并继续普通行动。成功后设置 `ceil(100 / frequencyPercent)` 次怪物自身行动冷却；冷却行动只递减计数，不抽任何施法 RNG。剩余冷却进入 Actor save、离层 floor、replay 和 state hash Schema v37。

v87 在频率骰之后纯计算每个候选的主目标、footprint、拒绝原因和有效权重。自疗的 20% 伤势阈值/最高四倍权重、状态存在/强度/免疫、三格距离倍率、召唤空间和次级实体风险均不抽 RNG；频率通过时仍只按声明顺序的有效权重抽一次选择骰。范围/射线/锥形只在被选中后抽共享基础伤害骰；召唤位置按距离、`y`、`x` 取前 N 个，不抽位置骰。怪物 owner 的限时召唤进入既有 summon save/hash 并执行 hostile AI。协议新增的候选列表、footprint 和拒绝原因只属于事件，state hash 保持 Schema v37。

v88 对每个攻击能力按距离、玩家优先级和稳定 ID 尝试玩家或玩家召唤物；目标检查、敌我 footprint 计数与有效权重计算不抽 RNG。多目标能力选中后仍只抽一次共享基础伤害骰，再按稳定目标顺序执行衰减和抗性。保持距离/受伤撤退只扫描八个相邻格，按最大最小敌距与固定方向顺序选择，不抽移动骰；没有更远格时回退普通行动。smart caster 只在效果实际作用于玩家后写入当时抗性，首次决策、频率失败、墙体拒绝和命中召唤物均不更新。观察记忆进入 save/replay 与 state hash Schema v38；旧存档默认空记忆且不推进 RNG。

v90 的资源获得、衰减与上限计算不抽 RNG：近战命中/击杀按内容数值即时增加对应池并发出 `resource.gained`；推进世界时间的行动内未被获得、消费或恢复触碰的池在结算后静默衰减，被拒绝的施法回合同样衰减；技法失败率按 profile 主宰属性确定性计算，只有失败率百分比掷骰照常消耗 RNG。

v89 的全局召唤指令切换不推进 `turn/worldTick`、能量、生命周期、冷却或 RNG。召唤物在自身能量行动中按稳定距离/ID 选敌，按固定八方向移动；只有近战命中和伤害复用现有 RNG。楼层切换按实体 ID 提取玩家 2 格内召唤物，在目标点半径 5 内按距离/坐标稳定落位；Guard 锚点重置为到达点。指令和锚点进入 save/replay 与 state hash Schema v39，旧存档默认 Follow。
