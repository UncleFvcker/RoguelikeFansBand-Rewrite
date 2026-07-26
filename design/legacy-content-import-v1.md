# 旧版内容导入管线 v1

状态：f_info/r_info 首刀、多 blow → meleeRoutine、状态/治疗/位移/直伤 bolt+ball 的 S: 法术映射已实现；导入产物只存在于本地 `.local/`，仓库与发行包继续只含原创内容。

## 1. 边界

- `rfb-legacy-import import-content <输出目录>` 通过 `RFB_LEGACY_SOURCE` 只读访问本地旧仓库，经 git 对象读取固定 commit（`191f48c3…`，先 rev-parse 校验）下的 `lib/edit/f_info.txt` 与 `lib/edit/r_info.txt`；绝不读取工作树、绝不写入旧仓库，输出目录不得位于旧仓库内。
- 产物是 rfb-content 格式的 terrain/actor JSON 片段加 `pack.json`，写入调用方指定目录（约定 `.local/packs/rfb-legacy/`，已被 gitignore）。产物可直接通过 `rfb-contentc inspect-source` 的全部校验并获得确定性 content hash。
- 不可表达的字段聚合进 `import-report.json` 缺口报告：按缺失法术形态、blow 效果、怪物/地形 flag 与跳过原因计数。**报告是后续规则族排期的数据来源。**
- 单元测试只使用原创合成样本；任何旧版名称、数值或文本都不进入仓库。协议、内容包与契约基线本轮不变。

## 2. v1 映射规则

- 行格式按固定 commit 的 `init1.c` 解析器钉死：`I:速度:骰d面:警觉:AC:沉睡:体重`、`W:深度:稀有度:…`、`B:方式:效果(骰)…`（`HURT(2d6)` 型为主，`DAM(…)` 是少数）、`F:`/`S:` 竖线分隔多行累积、`N:`/`G:`/`E:` 身份与字形。
- 怪物：速度原值（同为 110 基准）；`maxHp = 骰数×(面+1)/2` 向下取整、至少 1；`defense = AC/10`（对应核心 rating×10 的反向）；`attack = max(1, 深度/4)`（v1 显式近似）；取首个带骰效果为伤害骰，效果 token 逐 blow 映射伤害类型（HURT/DAM→物理，POISON/FIRE/COLD/ACID/ELEC→对应元素，其余物理并计入缺口）；**全部带骰 blow 映射为 `meleeRoutine`**（≥2 段时发射，`methodId = rfb-legacy.blow.<方式>`，`toHit` 沿用 demo 惯例 20，schema 上限 8 段而原版至多 4 段故无截断）；无骰 blow 与未映射法术、全部 flag 进缺口报告；经验值 = 深度×10（近似）。`S:` 行的 `1_IN_N` 转为 `frequencyPercent = 100/N`；SCARE/SLOW（实体目标状态）、HASTE（自身加速）、HEAL（按 3×深度 5–300 去重共享）映射为生成能力与 `monsterCasting` 块，能力挂在存根资源 `rfb-legacy.resource.essence` 上（怪物施法不消耗资源）。
- 地形：`MOVE→walkable`、`LOS 缺失→blocksSight`；tag 转 kebab ID；占位条目与无字形条目跳过。
- ID 命名空间 `rfb-legacy.*`，nameKey/descriptionKey 生成规范键；重名追加原始序号去重。

## 3. 首次全量导入结果（本地实测）

地形 180/188；怪物 1332/1396（95.4%），跳过 64 条无可表达近战的条目。1124 条怪物已带完整多段 `meleeRoutine`，107 条仍有无骰副攻（TOUCH/GAZE/WAIL 类）；截至 P42（诅咒族）已有 832 条生成 `monsterCasting`，法术映射累计 4337 实例（吐息、召唤、异种元素、心灵 248、诅咒 240），抗性档 1023 只怪 3842 条，`FREQ_N` 频率语法已解析，附身组 522 实例入 notApplicableSpells 桶，类型旗标（UNDEAD/DEMON/DRAGON/ANIMAL）折算为 actor 标签，blow 元素名（DISENCHANT/TIME/NETHER 等）直映伤害类型，施法表上限 64、源包文件预算 4096。缺口报告当前优先级（按覆盖数）：S_ 特殊/字形子类 177、TELE_OTHER/LEVEL 94、DARKNESS 85、DRAIN_MANA 83、AMNESIA 64、ANIM_DEAD 58、DISPEL 48、ANTI_MAGIC 47、HAND_DOOM 30；blow 效果缺口以 DRAIN_MANA、SHATTER、VAMP、CONFUSE 等效果语义为首（326 实例）；flag 缺口以 BASH_DOOR、DROP_CORPSE、NO_CONF/NO_SLEEP、FORCE_MAXHP、RES_TELE/RES_WALL/HURT_ROCK 为首。

## 4. v2 方向

k_info 物品导入；异种元素随伤害类型扩展落地后映射；字形子类召唤（S_HOUND/S_SPIDER 等）可按 glyph 派生标签；E: 中文名导出为本地 Fluent 片段；按报告落地新规则族后重跑导入提升表达率。
