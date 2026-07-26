# Contract v95：按类别召唤（summon-category）与召唤族导入

状态：历史 baseline；当前 active baseline 见 [contract-v96](contract-v96-resistance-profiles.md)。协议 1.95，内容包 1.86.0（content hash `01b74e86466aa5abfe682443819379504dde2efdf5d67d126fc3f1d20eb197a4`）；save 容器继续 v1；类别召唤复用既有 SummonIdentity 权威结构（存档校验同步接受类别召唤物：kind 带类别标签且等级 ≤ 上限），state hash 沿用 Schema v40。该 baseline 共 308 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

S\_ 召唤族是缺口报告最大剩余单族（32 种 token、670 实例）。原版语义（`monspell.c` `_summon()` / `summon_specific`）：按**类型**（恶魔/不死/龙/同类……）与**召唤等级 = 施法者种族等级**随机挑选具体怪物，数量掷默认骰（多数 1d3+1、同类 1d2+1、HI 系 1d3），落点优先玩家附近否则施法者附近，召唤物永久存在。

现有 `summon` 效果只支持固定 actor kind（v82 玩家侧 / v87 敌对侧）。本轮新增 `summon-category`：候选 = 携带指定**标签**且等级 ≤ 上限的怪物定义，逐只有界抽取；落位沿用既有召唤机制（自身半径规范序、零 RNG）。首版仅限 monsterCasting（玩家规划层拒绝，v91/v94 同款）。S_KIN 直接用既有固定 `summon` 映射为「召唤同类」，零新机制。

已知中性化差异（记录不复刻）：落点近己而非近玩家（沿用 v82 约定）；HI\_ 系在原版是更深怪物类选择，折算为同类别同等级上限的近似；导入召唤时长取上限 10 000 玩家回合近似原版永久。

## 2. 内容格式（1.86.0）

新效果 `summon-category { category, maximumLevel: u16, countDice: u8, countSides: u8, countBonus: u8, radius: u8, durationTurns: u16 }`：

- category 与 Detect 同字符集（小写/数字/`-_`、≤64），交叉校验须命中至少一个 actor 标签；
- maximumLevel 1–1000；countDice 1–8、countSides 1–8、countBonus ≤8 且最大可能数量 `dice×sides+bonus` ≤8（与既有 summon 上限一致）；radius 1–8；durationTurns 1–10 000；
- 目标规则/怪物白名单与 `summon` 相同（仅 self、range 0、无 LOE）；不进 Sequence。

demo 接入：新增 `demo.actor.mote-binder`（聚尘唤灵者，速度 100，maxHp 9），双能力：

- `demo.ability.mote-call`：summon-category「elemental」maxLevel 1、1d2 只、半径 2、时长 5（权重 2）——候选恰为既有五只 1 级元素小怪（ember-mote/storm-spark/frost-wisp/acid-seep/venom-spore），**零触碰既有内容**；
- `demo.ability.cantor-call`：summon-category「caster」maxLevel 1（权重 1）——「caster」标签存在（echo-cantor，4 级）但高于上限，运行时恒以 no-candidates 被拒，把空候选边界钉进契约。（设计期原拟用「echo」标签，经核实该标签有两只 1 级怪，遂改用唯一成员为 4 级的「caster」。）

## 3. 执行与 RNG 边界

- 规划零 RNG：候选 kind 按 id 排序（BTree 序）收集，空 → 新拒绝原因 `no-candidates`；开阔格 `open_positions_around` 规范序收集，取前 `dice×sides+bonus` 个，一个都没有 → `no-space`；
- 执行：数量 = `roll(countDice d countSides) + countBonus`（复用伤害骰掷法，countDice 次抽取），与可用格数取小（空间紧张按可用格截断，至少 1 格由规划保证）；每只召唤一次 `bounded(候选数)` 抽 kind；生成走既有 `actor_from_runtime_spawn` + `SummonIdentity`（owner/来源能力/剩余回合），敌对、按玩家回合到期，与 v87 完全一致；
- 频率骰、加权选择骰、逆频率冷却零改动。

## 4. 协议与事件（1.95）

- `AbilityEffectSpecDto` 新增 `summon-category` 变体；
- `MonsterAbilityRejectionReasonDto` 新增 `no-candidates`（决策事件诊断用，web 不渲染拒绝原因，零文案负担）；
- `AbilitySummonResolutionDto` 新增 `summonedKindIds: Vec<String>`（serde 默认空、空则跳过序列化——既有固定召唤序列化字节不变）；类别召唤时 `actorKindId` 携带类别标签、`summonedKindIds` 逐只列出实际 kind；
- Web：召唤消息在 `summonedKindIds` 非空时改用实际 kind 名列表，固定召唤路径不变。

## 5. 导入器映射（随后小步）

1. **类型旗标 → actor 标签**：UNDEAD→`undead`、DEMON→`demon`、DRAGON→`dragon`、ANIMAL→`animal`（其余旗标继续留在 unmappedMonsterFlags）；
2. **S\_ token → summon-category**（数量默认按 `_summon_parm`；显式 `(XdY)` 覆盖复用骰解析；maximumLevel = 施法者等级；半径 2、时长 10 000）：

| token | 类别 | 默认数量 |
| --- | --- | --- |
| S_MONSTER | legacy-import（任意） | 1d3+1 |
| S_UNDEAD / S_HI_UNDEAD | undead | 1d3+1 / 1d3 |
| S_DEMON / S_HI_DEMON | demon | 1d3+1 / 1d3 |
| S_DRAGON / S_HI_DRAGON | dragon | 1d3+1 / 1d3 |
| S_ANIMAL | animal | 1d3+1 |

3. **S_KIN → 既有固定 `summon`**：actorKindId = 施法者自身 kind、2 只、半径 2、时长 10 000，id 按施法者去重（`rfb-legacy.ability.kin-<caster>`）。

实际收割 **493 实例**（与预估一致：MONSTER 131 + UNDEAD 116 + DEMON 106 + DRAGON 56 + ANIMAL 8 + KIN 76），施法怪 765→783、法术映射累计 3071、未映射 2650→2157；S_SPECIAL/UNIQUE/CYBER/PANTHEON/AMBERITE/ANGEL（无旗标）与 HOUND/SPIDER 等字形子类（共 177）留缺口。基建同轮调整：源包文件预算 `MAX_SOURCE_FILES` 2048→4096（召唤/直伤的按等级能力变体使产物达 2049 文件；读取期守卫，零序列化影响）；`DomainEvent::MonsterAbilityCast` 载荷装箱消除 clippy 枚举尺寸告警（纯表示层）。

## 6. 契约场景（v95）

迁移 306 条（完全零语义漂移）后新增 307-308 共 2 条：

- 307 类别召唤成功（seed 0）：mote-call 掷 1d2 得 1 只（venom-spore 经有界抽取入场，带 SummonIdentity）；同一决策事件里 cantor-call 以 `no-candidates` 被拒，空候选边界同场自证；
- 308 数量骰对照（seed 3）：1d2 掷得 2 只（双 storm-spark——逐只独立抽取允许同类重复），数量骰语义入约。

全部场景 saveRoundTrip。类别/上限过滤、kind 抽取、空候选拒绝由核心单元测试覆盖；类型旗标标签、S_ 默认骰、`(XdY)` 覆盖、S_KIN 映射由导入器单元测试覆盖。

## 7. 验证

常规全套 + `migrate-baseline` 零漂移核对 + 新场景 `refresh` 人工审阅；clippy 单跑验退出码；五件套同步（协议 1.95 / pack 1.86.0 / content.lock / BUILT_IN+PREVIOUS / README）；本地桌面 E2E（contentVisualCount 74→75）。
