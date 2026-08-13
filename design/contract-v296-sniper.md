# Contract v296：正式狙击手职业

状态：已实现。Protocol `1.198`，State Hash Schema `v98`，内容包 `1.313.0`，save v1；
content hash `8b89d37d689db0c180feb1dbe213a3aa30aef910bd72a12a6c3d1af8222296dc`。

## 正式内容与 ID

- 新增 `demo.class.sniper`、`demo.skill-set.sniper`、`demo.build.sniper` 与
  `demo.actor.sniper-player`。
- 新增 `demo.ability.sniper-*` 与同名 `demo.ability-program.sniper-*` 共 17 对：专注、十五种
  特殊射击和探测怪物。具体后缀为 `concentrate`、`shining-arrow`、`retreat-shot`、
  `disarming-shot`、`burning-shot`、`shatter-shot`、`freezing-shot`、`knockback-shot`、
  `piercing-shot`、`evil-shot`、`holy-shot`、`exploding-shot`、`double-shot`、
  `thunder-shot`、`needle-shot`、`saint-stars-arrow`、`probe-monsters`。
- 不新增 item、affix、resource、material 或 ability book ID；出生复用匕首、软皮甲、轻弩
  与 20–30 支弩栓。

## RFB 职业数据

- 属性 `+2/-1/-1/+2/+1/0`，life 100%、base HP 4、经验 110%、宠物维持除数 40。
- 八项技能基础为 `25/24/28/5/32/18/35/72`，成长为
  `12/10/10/0/0/0/12/28`。
- `master:lib/edit/s_info.txt` N:27 映射为普通武器默认 `2000/4000`；匕首
  `4000/4000`；投石索、短弓、长弓、轻弩和重弩 `4000/8000`；骑术 `0/0`。权威审计现
  覆盖 6 个正式职业和 68 种基础武器。
- 中文职业名和十七个能力名取自 `master:src/sniper.c` 与 `master:src/spells_m.c`。

## 能力与 UI

- 十六项狙击能力按原版等级和专注门槛绑定；探测怪物为 15 级、INT、20 HP、基础失败率
  80。执行继续复用 contract-v293 至 v295 的专注、统一 projectile 与怪物探测事务。
- 新游戏开放 `demo.build.sniper`；职业能力面板将十六项狙击能力聚合到“狙击”分组，
  探测怪物保留独立职业能力；三套 tileset 均登记 player actor。
- 原版 `CLASS_SENSE1_SLOW | CLASS_SENSE1_STRONG` 依赖尚不存在的通用装备感知系统，记为
  共享缺口；本批不在 Sniper 核心中硬编码伪鉴定。

## 版本与验收

- 本批只增加内容和 New Game UI，不改变协议、save 或 state-hash 输入。active baseline
  标记为 `contract-v296`；现有 26 条 fixture 不选择狙击手构筑，因此只复验、不刷新。
- 聚焦测试覆盖正式出生身份、属性/技能、逐武器熟练度、随机弩栓数量、17 项职业绑定和
  Sniper profile；内容编译、双语本地化、Web 选择列表和 tileset 映射纳入常规验证。
