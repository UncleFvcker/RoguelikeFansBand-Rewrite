# RoguelikeFansBand 本地化与中文文本重构计划

状态：初版规划

目标仓库：`UncleFvcker/RoguelikeFansBand-Rewrite`

关联文档：[HTML/Rust 重构计划](html-rewrite-plan.md)

本文档用于持续处理三类问题：未完全汉化的英文文本、按英文结构硬翻造成的中文语序问题，以及大量散落在规则代码中的硬编码用户文本。

## 1. 目标

- 用户可见文本不再直接硬编码在 Rust、TypeScript 或遗留 C 规则代码中；
- 游戏核心输出结构化事件和稳定消息 ID，不输出拼接完成的英文句子；
- 英文和简体中文使用独立语言资源，中文可以自由调整参数顺序和整句结构；
- 怪物、物品、神器、职业、法术、地形等内容使用稳定语义 ID；
- 术语、标点、颜色和专名有统一规范；
- 未翻译、占位符错误、废弃 key 和新的硬编码文本能够由 CI 自动发现；
- 当前源码和数据文件是唯一文本事实来源；旧 `localization/` 文本不得自动导入新系统。

## 2. 文本基线原则

`localization/` 中的旧 TSV、旧译文、完成率报告和审阅结果已经落后于当前游戏，不作为新本地化系统的输入数据，也不用于统计当前汉化完成度。

新基线必须从当前分支重新生成，扫描范围至少包括：

- `src/` 中当前仍可到达的用户消息、提示、名称和描述；
- `lib/edit/` 中当前实际加载的数据；
- `lib/help/text/` 中当前发布的帮助文本；
- 配置、按键、菜单和平台前端文本；
- 未来新增的 Rust、TypeScript、Tauri 和 tileset manifest；
- 生成的 `*_name_zh.inc` 与其真实上游数据来源。

第一次审计固定使用旧仓库标签 `v1.3.0.7`、commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`。以后升级基线必须记录新的 commit，不允许用一个不断移动的“当前源码”覆盖旧报告。

旧 `localization/` 目录只允许用于：

- 参考过去使用过的脚本结构；
- 人工查询历史译名；
- 在明确逐条确认后作为翻译记忆候选。

禁止直接批量回填旧 TSV。所有新审计记录必须包含当前文件哈希、源码位置、上下文和本次扫描版本；源码发生变化后，旧记录自动标为 stale。

## 3. 核心问题

### 3.1 文本与规则代码耦合

当前常见形式：

```c
msg_format("You feel the %s (%c) you are wearing %s %s...", ...);
```

直接替换字符串仍会继承英文参数顺序，容易得到不自然中文。未来核心应改为：

```rust
GameEvent::ItemSensed {
    item: ItemRef,
    slot: SlotId,
    feeling: FeelingId,
}
```

前端再按当前语言生成完整句子。

### 3.2 句子由多个碎片拼接

英文可用前缀、动词和后缀拼接，中文通常需要重排整句。禁止继续增加：

```text
"你的 " + item + verb + target
```

应使用一条完整模板和具名参数：

```ftl
combat-slay = 你的{ $item }斩杀了{ $target }。
combat-covered = 你的{ $item }被{ $effect }包裹。
```

### 3.3 名称同时承担逻辑 ID

逻辑判断不能依赖英文名称、中文名称或显示文本。所有内容必须使用稳定 ID：

```text
monster.red_dragon
item.long_sword
artifact.vorpal_blade
spell.fire_ball
terrain.granite_wall
```

显示名称只是本地化资源。

## 4. 本地化技术方案

采用 Mozilla Fluent 作为主要消息格式：

- Rust：`fluent-bundle`；
- TypeScript：`@fluent/bundle`；
- 资源文件：FTL；
- 默认语言：`en-US`；
- 当前主要翻译：`zh-CN`。

选择 Fluent 的原因：

- 支持具名参数和语言独立的参数顺序；
- 支持选择表达式和复数/语法变体；
- Rust 与 TypeScript 均有成熟实现；
- 翻译者可调整完整句子，不需要修改代码；
- 比直接使用 printf、JSON 字符串和字符串拼接更适合复杂游戏文本。

目录建议：

```text
locales/
├─ en-US/
│  ├─ combat.ftl
│  ├─ items.ftl
│  ├─ monsters.ftl
│  ├─ ui.ftl
│  ├─ quests.ftl
│  └─ errors.ftl
├─ zh-CN/
│  ├─ combat.ftl
│  ├─ items.ftl
│  ├─ monsters.ftl
│  ├─ ui.ftl
│  ├─ quests.ftl
│  └─ errors.ftl
└─ glossary/
   ├─ terms.zh-CN.yml
   ├─ proper-names.zh-CN.yml
   └─ style-guide.zh-CN.md
```

## 5. 消息 ID 与事件模型

消息 ID 使用稳定的语义命名，不使用英文原文作为 key：

```text
combat.melee.hit
combat.melee.miss
combat.slay.weapon
item.sense.equipped
item.pickup.success
quest.leave.confirm
ui.inventory.title
error.save.unsupported-version
```

游戏核心输出结构化事件：

```rust
pub enum GameEvent {
    MeleeHit {
        attacker: ActorRef,
        defender: ActorRef,
        damage: i32,
        critical: Option<CriticalKind>,
    },
    ItemSensed {
        item: ItemRef,
        location: ItemLocationRef,
        feeling: FeelingId,
    },
}
```

协议层序列化为 DTO，TypeScript 前端根据事件类型选择 Fluent key。事件本身不能保存已经本地化的句子。

只有以下文本允许由核心直接提供：

- 玩家输入的自定义名称；
- 模组提供但尚未声明本地化 key 的用户内容；
- 调试/崩溃诊断中的纯技术信息。

## 6. 参数与中文语序规范

### 6.1 使用具名参数

禁止 `%s %s %s` 这种无法理解语义的接口。使用：

```ts
{
  item: LocalizedEntityRef,
  target: LocalizedEntityRef,
  effect: EffectId,
  count: number
}
```

### 6.2 模板必须是完整句子

翻译资源负责完整标点、语序和助词。不要从代码传入“的”“了”“被”等语法碎片。

### 6.3 实体引用携带语义，不携带英语语法

实体引用可以包含：稳定内容 ID、是否已知/可见、唯一怪物标记、数量、所有权以及是否玩家本人。

不要传递英文主格、宾格、冠词等已经渲染完成的字符串。中文渲染器按自身需要选择“你”“它”“某个怪物”或正式名称。

### 6.4 避免滥用通用万能模板

如果两个语言场景在语义上不同，应使用两个 key，而不是强行设计一个几十个参数的模板。

## 7. 富文本和颜色

现有 `<color:R>` 标记在迁移期间继续兼容，但新系统不应让翻译文件直接生成任意 HTML。

建议消息格式化结果为安全 token：

```ts
type RichTextToken =
  | { type: "text"; value: string }
  | { type: "entity"; id: string; text: string }
  | { type: "emphasis"; style: TextStyle; children: RichTextToken[] }
  | { type: "key"; command: CommandId; text: string };
```

HTML 前端把 token 转成安全 DOM；终端兼容层可把 token 转成旧颜色代码。禁止从翻译文件注入任意 HTML、脚本或未经校验的标签。

## 8. 内容名称与描述

名称类资源与运行时消息分开，包括怪物、物品、神器、技能、法术、种族、职业、性格、地形、任务、地下城和状态效果。

数据文件只引用内容 ID：

```json
{
  "id": "monster.red_dragon",
  "nameKey": "monster-red-dragon-name",
  "descriptionKey": "monster-red-dragon-description"
}
```

现有 `*_name_zh.inc` 和 `lib/edit/*.txt` 翻译通过生成脚本转换，不手工复制多份。

## 9. 中文术语和风格指南

建立并强制维护：

- 专名表和机制术语表；
- 怪物/物品命名规则；
- 全角/半角标点规则；
- 数字、百分比、骰子和属性格式；
- 引号、书名号和括号规范；
- 英文缩写保留规则；
- “你/玩家/角色”的使用场景；
- 简体中文地区用词；
- 系统、战斗、幽默、任务和技术错误的语气等级。

同一个术语只保留一个首选译名，备选译名记录为别名，供旧存档搜索和翻译记忆使用。

## 10. 工具链

建立面向当前代码的新工具链。可以复用旧脚本中的解析思路，但不能默认信任旧输出数据：

1. `extract`：扫描 C、Rust、TypeScript、数据文件中的用户文本；
2. `classify`：区分用户文本、技术字符串、格式串、资源路径和内部 ID；
3. `candidate-import`：只把人工确认过的历史译文作为候选，不直接写入 Fluent；
4. `lint`：检查缺失 key、重复 key、参数不一致和非法富文本；
5. `unused`：发现废弃消息；
6. `hardcoded-check`：阻止新用户文本直接进入规则代码；
7. `glossary-check`：发现禁用译名和术语不一致；
8. `pseudo-locale`：生成加长伪语言，测试布局和遗漏；
9. `report`：输出完成率、审阅队列和高频问题。

CI 最低检查：英中 key 集合一致、具名参数一致、FTL 可解析、富文本合法、不出现新的高置信用户硬编码文本、代码不按英文原文查找逻辑对象。

## 11. 迁移阶段

### 阶段 0：冻结和分类现有资产

- 对当前代码执行一次全新扫描，不读取旧 TSV 的翻译列；
- 为扫描结果记录源文件哈希、代码上下文和当前 commit；
- 增加状态：未处理、机器初译、人工翻译、语序待改、术语待审、已确认、技术文本；
- 去重但保留每个源码出现位置；
- 把高频运行时消息与低频调试文本分开；
- 建立中文风格指南 v1。

验收：当前可发布代码中的每条用户文本都有基于当前源码生成的稳定审计 ID 和处理状态，且报告不混入 stale 数据。

### 阶段 1：建立 Fluent 和消息协议

- 创建 `locales/en-US`、`locales/zh-CN`；
- 建立 Rust/TypeScript Fluent 加载器；
- 定义消息 ID 命名规则；
- 定义 `GameEvent → message key + args` 映射；
- 支持语言切换和英文回退；
- 支持开发模式显示缺失 key。

验收：20 条代表性消息可以在英文/中文之间切换，中文参数顺序与英文完全独立。

### 阶段 2：迁移高频 UI 和交互文本

优先迁移主消息栏、输入提示、背包、装备、商店、人物状态、目标选择、设置、按键和保存错误。

验收：HTML 前端主要操作流程不再依赖硬编码用户文本。

### 阶段 3：迁移战斗和规则消息

迁移命中、暴击、锋锐、杀戮、烙印、抗性、法术、状态、怪物行动、物品感知、鉴定和任务事件。这一阶段重点消灭英文式碎片拼接，按语义拆分事件。

验收：固定种子回放的所有用户消息均来自消息 key，中文句序通过人工审阅。

### 阶段 4：迁移内容名称和数据文件

- 为怪物、物品、神器、技能、法术、地形、任务建立稳定 ID；
- 从当前实际加载的数据和当前生成文件提取中文名称，并逐条确认；
- 解决同名、别名和历史译名；
- 数据文件停止同时保存逻辑英文名和显示中文名。

### 阶段 5：帮助文档

- 把帮助内容从程序消息资源中分离；
- 保留 Markdown/结构化文档源；
- 自动生成目录和交叉链接；
- 检查帮助中的按键、数值和机制引用；
- 对比英文更新，标记中文过期段落。

### 阶段 6：清除遗留硬编码

- C 版本只保留兼容层和技术诊断文本；
- Rust 核心零用户可见硬编码文本；
- TypeScript UI 零业务文案硬编码文本；
- CI 阻止回归；
- 清理已迁移的旧 include 和重复翻译表。

## 12. 测试策略

- Fluent 解析测试；
- 英中 key/参数一致性测试；
- 固定事件的英中快照测试；
- 中文语序人工金标准测试；
- 伪语言和超长文本布局测试；
- 中文宽字符、标点和换行测试；
- 富文本嵌套和颜色测试；
- 未知怪物、不可见实体和代词测试；
- 数量、单位和动态按键测试；
- 旧存档名称导入测试；
- tileset/消息面板分层截图测试。

## 13. 完成指标

- Rust 核心用户可见硬编码文本：0；
- TypeScript 业务文案硬编码文本：0；
- `en-US`/`zh-CN` 消息 key 覆盖率：100%；
- 占位参数不一致：0；
- 非法富文本：0；
- 高优先级未翻译运行时文本：0；
- 已知英文语序直译问题：0；
- 每次提交新增硬编码文本均由 CI 拦截；
- 所有专名和核心机制术语进入词典。

## 14. 第一里程碑

> 建立 Fluent 双语框架，把 20 条最典型且存在中文语序问题的消息改为结构化事件，并证明英文和中文可以使用完全不同的参数排列。

样本应覆盖玩家/怪物攻击、物品感知、拾取、装备、商店、状态变化、任务确认、存档错误和动态按键提示。

## 15. 当前进度与下一步

当前状态：游戏中已有大量中文文本，但缺少基于当前代码重新生成的可信审计基线，也尚未建立正式 Fluent 资源与结构化消息协议。

下一步：

1. 为当前代码编写全新扫描器输出格式和 stale 判定规则；
2. 编写 `zh-CN` 风格指南 v1；
3. 确定 Fluent key 命名规范；
4. 创建 `locales/en-US` 和 `locales/zh-CN`；
5. 建立 `rfb-protocol` 中的首批 `GameEvent`；
6. 选取 20 条语序问题作为试点；
7. 建立 key/参数/硬编码 CI 检查；
8. 将迁移进度持续更新到本文档。
