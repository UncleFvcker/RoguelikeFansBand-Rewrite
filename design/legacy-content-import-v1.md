# 旧版内容导入管线 v1

状态：f_info/r_info 首刀已实现；导入产物只存在于本地 `.local/`，仓库与发行包继续只含原创内容。

## 1. 边界

- `rfb-legacy-import import-content <输出目录>` 通过 `RFB_LEGACY_SOURCE` 只读访问本地旧仓库，经 git 对象读取固定 commit（`191f48c3…`，先 rev-parse 校验）下的 `lib/edit/f_info.txt` 与 `lib/edit/r_info.txt`；绝不读取工作树、绝不写入旧仓库，输出目录不得位于旧仓库内。
- 产物是 rfb-content 格式的 terrain/actor JSON 片段加 `pack.json`，写入调用方指定目录（约定 `.local/packs/rfb-legacy/`，已被 gitignore）。产物可直接通过 `rfb-contentc inspect-source` 的全部校验并获得确定性 content hash。
- 不可表达的字段聚合进 `import-report.json` 缺口报告：按缺失法术形态、blow 效果、怪物/地形 flag 与跳过原因计数。**报告是后续规则族排期的数据来源。**
- 单元测试只使用原创合成样本；任何旧版名称、数值或文本都不进入仓库。协议、内容包与契约基线本轮不变。

## 2. v1 映射规则

- 行格式按固定 commit 的 `init1.c` 解析器钉死：`I:速度:骰d面:警觉:AC:沉睡:体重`、`W:深度:稀有度:…`、`B:方式:效果(骰)…`（`HURT(2d6)` 型为主，`DAM(…)` 是少数）、`F:`/`S:` 竖线分隔多行累积、`N:`/`G:`/`E:` 身份与字形。
- 怪物：速度原值（同为 110 基准）；`maxHp = 骰数×(面+1)/2` 向下取整、至少 1；`defense = AC/10`（对应核心 rating×10 的反向）；`attack = max(1, 深度/4)`（v1 显式近似）；取首个带骰效果为伤害骰，效果 token 映射伤害类型（HURT/DAM→物理，POISON/FIRE/COLD/ACID/ELEC→对应元素，其余物理并计入缺口）；多 blow、全部法术、全部 flag 进缺口报告；经验值 = 深度×10（近似）。
- 地形：`MOVE→walkable`、`LOS 缺失→blocksSight`；tag 转 kebab ID；占位条目与无字形条目跳过。
- ID 命名空间 `rfb-legacy.*`，nameKey/descriptionKey 生成规范键；重名追加原始序号去重。

## 3. 首次全量导入结果（本地实测）

地形 180/188；怪物 1332/1396（95.4%），跳过 64 条无可表达近战的条目。958 条怪物携带尚不可表达的法术，1140 条有多段 blow。缺口报告给出的规则族优先级（按覆盖数）：SCARE/CONFUSE/BLIND 状态法术、HEAL/自愈、TELE_TO/BLINK 位移、DETECT 类、BRAIN_SMASH/DRAIN_MANA 特殊攻击；blow 效果缺口以 DRAIN_EXP、SHATTER、DISENCHANT、VAMP 为首；flag 缺口以 BASH_DOOR、DROP_CORPSE、NO_CONF/NO_SLEEP、FORCE_MAXHP 为首。

## 4. v2 方向

k_info 物品导入；多 blow → meleeRoutine；已支持形态的 S: 法术（自愈、直伤 bolt/breath 子集）映射到 monsterCasting；E: 中文名导出为本地 Fluent 片段；按报告落地新规则族后重跑导入提升表达率。
