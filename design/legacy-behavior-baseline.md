# 旧版行为基准与差分测试规范

状态：P0 规范已确定，首批 20 个原创 contract fixtures 和旧存档稳定前缀断言已建立

## 1. 基准版本

首个重构基准固定为旧仓库：

- 仓库：`UncleFvcker/RoguelikeFansBand-zh-CN`
- 标签：`v1.3.0.7`
- commit：`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`
- 发布平台基准：Windows x64
- 本地来源变量：`RFB_LEGACY_SOURCE`
- 当前默认路径：`D:/codex/Frogcomposband/master`

工具直接读取本地 Git 仓库，但必须通过 `RFB_LEGACY_REF=v1.3.0.7` 解析并校验上述 commit。默认使用 `git show`、`git archive` 或等价的 Git 对象读取方式，不能把当前工作树当作基准，因为其中可能存在未提交修改和规划分支提交。

工具对旧仓库只有读取权限，不能 checkout、clean、reset、修改或生成文件到旧仓库。临时导出统一写入新仓库被忽略的 `.local/legacy-cache/`。

如果以后更换基准版本，必须新增一个基准集合，不能覆盖已有结果。

## 2. 目的

行为基准用于回答三个问题：

1. Rust 模块是否保持了旧版规则；
2. 变化是预期修正，还是无意回归；
3. Windows、Linux、macOS 和 Android 原生核心是否产生相同结果。

基准不是要求新版复刻 Win32 GDI、终端缓存或旧版渲染缺陷。规则、可见性语义、消息事件和存档内容属于对照范围；像素级 Win32 绘制只作为视觉迁移参考。

## 3. 基准产物

计划目录：

```text
.local/legacy-baseline/
├─ manifest.json
├─ save-samples.json
├─ parsed-save-samples.json
├─ saves/
│  ├─ legacy-save-01.bin
│  ├─ legacy-save-02.bin
│  └─ legacy-save-03.bin
├─ scenarios/
│  ├─ movement/
│  ├─ combat/
│  ├─ items/
│  ├─ statuses/
│  ├─ magic/
│  ├─ monsters/
│  └─ saves/
└─ screenshots/

tests/fixtures/contract-v2/
├─ baseline-policy.json
├─ waivers/
│  └─ README.md
└─ scenarios/
   ├─ 01-move-north.json
   ├─ ...
   └─ 20-save-round-trip-after-combat.json
```

`manifest.json` 至少记录：旧仓库 commit、构建工具链、编译参数、平台、配置文件哈希、内容文件哈希、随机种子、场景版本和生成时间。

`.local/legacy-baseline/` 不进入 Git。仓库中只提交本项目原创、经过规范化且不包含旧文本、旧专名、完整数值表、截图、存档或其他旧表达内容的契约断言。

提交 fixture 中的 `legacyCommit` 只标识准备对照的旧版基准，不表示当前断言已经从旧版数据复制或完成差分验证。

旧版源码、二进制、文本、数据、截图、存档和素材不能复制到新仓库或新游戏发行包。公共 CI 没有 `RFB_LEGACY_SOURCE` 时跳过本地差分测试，只运行原创 contract fixtures。

本地样本通过以下命令登记：

```powershell
cargo run -p rfb-legacy-probe -- catalog-saves <旧存档1> <旧存档2> <旧存档3>
```

工具要求源文件位于已验证的 `RFB_LEGACY_SOURCE` 内，读取四字节版本头，复制为中性样本名，并在复制前后校验 SHA-256。当前本机集合包含两份 1.3.0.7 `baseline-exact` 样本和一份 1.2.0.6 `legacy-migration` 样本。

`rfb-legacy-import` 对样本执行旧格式的链式 XOR 解码，并只解析跨 1.2.0.6/1.3.0.7 稳定、且不依赖 C 结构体内存布局的前缀：

- 四字节版本；
- `sf_system`、保存时间、生命数和保存次数；
- RNG 位置及 63 项状态；
- 9 个基础选项字节、作弊标志、自动保存选项；
- 8 组 option/window flags 和 masks；
- 前缀消费长度、解码值校验和及编码字节校验和。

```powershell
cargo run -p rfb-legacy-import -- inspect-prefix .local/legacy-baseline/saves/legacy-save-01.bin
cargo run -p rfb-legacy-import -- record-catalog .local/legacy-baseline/save-samples.json
cargo run -p rfb-legacy-import -- verify-catalog .local/legacy-baseline/save-samples.json
```

稳定前缀固定解码 409 字节，连同版本头和 XOR seed 共消费 414 个编码字节。`record-catalog` 只创建一次本地 `parsed-save-samples.json`，之后由 `verify-catalog` 精确复验；目录元数据、样本版本、长度和 SHA-256 也同时校验。这不表示旧 `player_type`、物品、地图或完整存档已经可导入。

## 4. 场景格式

每个场景使用稳定 ID。当前 active `contract-v2` 的输入部分包含：

```json
{
  "schemaVersion": 1,
  "id": "combat.melee.basic-hit",
  "legacyCommit": "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9",
  "determinism": "exact",
  "seed": "0x0123456789abcdef",
  "preconditions": { "world": "demo.world.original-v1" },
  "commands": [],
  "saveRoundTrip": true,
  "assertions": {}
}
```

`rfb-contract` 执行命令后精确比较最终 revision、turn、command sequence、玩家位置、实体数量、事件顺序、changed cells、删除实体、结构化错误、state hash 和可选存档回环 hash。提交的 fixture 必须包含断言；`observe` 命令只输出实际观察结果，不会自动改写或批量刷新 golden：

```powershell
cargo run -p rfb-contract -- observe tests/fixtures/contract-v2/scenarios/01-move-north.json
cargo run -p rfb-contract -- verify tests/fixtures/contract-v2/scenarios/01-move-north.json
```

场景不能依赖屏幕坐标、数组下标或本地化后的名称定位对象。对象使用稳定测试 ID；显示文本单独由本地化测试验证。

## 5. 首批覆盖矩阵

阶段 0 至少覆盖：

- 八方向移动、等待、开门、上下楼和碰撞；
- 基础近战、命中、闪避、暴击、锋锐、抗性与伤害结算；
- 物品生成、拾取、堆叠、装备、感知、鉴定和销毁；
- 中毒、眩晕、恐惧、加速、减速和持续时间递减；
- 投射、范围法术、召唤、传送和目标选择；
- 怪物生成、视野、寻路、施法和回合顺序；
- 地图生成的结构性断言；
- 旧存档读取、保存回环和关键字段校验；
- `map_info()` 输出的地形、实体、可见性、记忆和光照语义；
- 地图与消息区的代表性截图，用于分层迁移参考。

## 6. 确定性等级

测试分为三类：

1. `exact`：命令、事件、状态和 RNG 消耗必须完全相同；
2. `semantic`：允许内部顺序或 ID 不同，但规则结果必须相同；
3. `visual-reference`：仅用于人工或截图差异审查，不约束新版像素实现。

旧版随机数实现尚未迁移时，可以先使用 `semantic` 场景。迁移相关模块后，应尽量升级为 `exact`。

## 7. 差分规则

- 快照比较前先移除时间戳、窗口尺寸、平台路径等非确定字段；
- 无序集合按稳定 ID 排序后比较；
- 浮点视觉参数不进入规则基准；
- 任何预期规则变化必须增加带原因的差异豁免；
- 差异豁免必须包含旧结果、新结果、批准日期和关联 issue；
- 不允许通过更新全部 golden 文件来隐藏无法解释的变化。

完整审批流程、机器验证规则和 waiver v1 格式见[Contract 基准更新与差异豁免政策](baseline-update-policy.md)。当前没有已批准差异。

`rfb-contract` 的快照规范化 Schema v1：

- 递归按 key 规范化 JSON object；
- `entities`、`items`、`statuses` 按稳定 ID 排序；
- `cells`、`changedCells` 按 y/x 坐标排序；
- `removedEntities` 等无序 ID 集合按字符串排序；
- 保留事件和命令数组顺序；
- 移除时间戳、session/request ID、本地源路径、平台和窗口信息；
- 统一 CRLF/CR 为 LF；
- 拒绝权威快照中的浮点数；
- 在规范化输出中写入 `normalizationSchemaVersion`。

```powershell
cargo run -p rfb-contract -- normalize-snapshot <snapshot.json>
cargo run -p rfb-contract -- hash-snapshot <snapshot.json>
```

## 8. 阶段 0 完成门槛

进入大规模 Rust 规则实现前，必须完成：

- 基准 manifest 生成器；
- 至少 20 个代表性规则场景；
- 至少 3 个仅保存在本机 `.local/` 中的旧存档导入样本；
- 命令回放格式 v1；
- 状态快照规范化工具；
- 所有原生目标共用的差分测试入口；
- 基准更新审批规则。

允许在阶段 0 同时创建最小 Cargo workspace 和测试工具，但不能在没有基准的情况下批量翻译规则模块。

当前完成情况：

- 已完成：基准 manifest 探针、20 个原创 exact contract fixtures、所有原生目标可共用的 `rfb-contract` 测试入口；
- 已完成：回放文件 v1、每 100 命令和最终状态检查点、10,000 回合无漂移测试、存档重载续播测试；
- 已完成：3 个本地旧存档样本及 SHA-256/版本头清单、快照规范化 Schema v1 和 CLI；
- 已完成：baseline policy v1、diff waiver v1 格式和 CI 验证；
- 已完成：旧存档链式 XOR 解码、409 字节稳定前缀解析及 3 个本地样本的字段级断言；
- 待完成：从稳定前缀继续扩展隔离的完整旧存档导入和结构化转换报告；
- 当前 fixture 只固定已经实现的原创垂直切片行为，不代表物品、状态、法术、AI 等旧 RFB 模块已经迁移。
