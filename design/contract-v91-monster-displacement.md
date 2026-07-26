# Contract v91：怪物位移法术族

状态：当前 active baseline。协议 1.91，内容包 1.82.0（content hash `81e4e9d5f14d5a6e9990db8a6b1a60623eba81279c288b266d3274cfee523916`）；save 容器继续 v1；位移不新增权威状态字段，state hash 沿用 Schema v40。active baseline 共 288 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

FrogComposband 怪物法术中 BLINK（短距自闪）、TELE_SELF/TELEPORT（远距脱离）与 TELE_TO（把玩家拉到身边）是覆盖面最大的位移三件套（导入报告：142/140/173 只怪物）。本轮以原创中性机制实现这三种形态；TELE_AWAY（推走玩家）、TELE_LEVEL、传送抗性（RES_TELE）与念力封锁留给后续。

## 2. 内容格式（1.82.0）

`AbilityEffectDefinition` 新增三个效果：

- `blink-self { radius: u8 }`：施法者短距自闪，半径 1–10；
- `teleport-self { minimumDistance: u8 }`：施法者跨图脱离，落点与当前威胁（玩家）至少该距离，1–64；
- `teleport-target { }`：把目标（玩家阵营）拉到施法者相邻格。

目标规则：`blink-self`/`teleport-self` 仅 `self` 目标；`teleport-target` 走既有 projectile 规则（entity/position + 射程 + LOE）。三者首版仅用于 `monsterCasting`（内容校验拒绝进入玩家能力书/技法），玩家侧位移继续使用既有 `teleport`。

demo 接入：三种效果全部由新增的 `demo.actor.rift-stalker`（裂隙潜行者）承载（rift-drag 权重 2、rift-escape 权重 1、echo-slip 权重 1）。刻意不改动 Echo Cantor 的能力表：向既有加权池插入新条目会移动全部 243-265 历史场景的选择骰映射，实测会让十余条场景偏离各自命名主题；新怪物承载新形态则保持历史基线零语义漂移。

## 3. 执行与 RNG 边界

- 候选格枚举一律行优先规范序，只接受地图内、可行走、无存活 actor、非玩家占据；施法者自身当前格排除。
- `blink-self`：半径内候选集；**一次有界 RNG 抽取**选落点（与 vault 落位选择同精神）；空候选 → 该能力在候选过滤阶段被拒（同 clean-shot/召唤无空间语义，不消耗选择骰之外的 RNG）。
- `teleport-self`：全图候选中筛 `Chebyshev(落点, 玩家) >= minimumDistance`；一次有界抽取；空候选回退为把 minimumDistance 减半再筛一次，仍空则拒绝。
- `teleport-target`：拉取目标到施法者八邻的第一个合法格（规范序，**零 RNG**）；无空格 → 候选阶段拒绝。玩家被拉动复用 `relocate_player` 管线（到达触发陷阱/被动感知/可见性刷新照常）。
- 频率骰、加权选择骰、逆频率冷却与 v86–v88 完全一致；位移效果本身除上述抽取外不掷伤害骰。
- 怪物位移后 pack/summon 身份、状态、冷却全部随体移动（只是位置变更）；离层不可达，位移仅限当前层。

## 4. 协议与事件（1.91）

- 事件 `monster.blinked` / `monster.teleported`（args: source）与 `monster.dragged-target`（args: source/target），三者共用 outcome `MonsterDisplacementResolutionDto { actorId, from, to }`（actorId 为被位移者：闪现/脱离时是施法者，拖拽时是目标）；被拖目标可以是玩家或玩家召唤物，玩家被拖复用 `relocate_player`（到达陷阱照常触发）；
- changed cells 覆盖起点与终点；玩家被拉时照常输出可见性/visual 变更。
- Web：三条事件的中英文案；无新面板。

## 5. 导入器映射（随后小步）

BLINK → `rfb-legacy.ability.blink`（blink-self r10）；TELE_SELF/TELEPORT → `rfb-legacy.ability.escape`（teleport-self min 10）；TELE_TO → `rfb-legacy.ability.drag`（teleport-target r8）。已实现并重跑：casting 怪物 454 → 553，位移映射 BLINK 142 / TELE_SELF 140 / TELE_TO 173，共享能力 81 个。

## 6. 契约场景（v91）

迁移 282 条（零语义漂移，仅哈希更新）后新增 283-288 共 6 条：blink 成功、escape 脱离（落点距玩家 >=8）、拖拽玩家、拖拽落点触发隐藏陷阱、拖拽玩家召唤物（玩家超距时按稳定序选中召唤物）、七墙一玩家围死时 drag 以 no-space 被拒且回退施放 blink。teleport-self 减半回退与 blink 全封锁拒绝由核心单元测试覆盖（全墙小图构造）。全部场景 saveRoundTrip。

## 7. 验证

常规全套 + `migrate-baseline` 零语义漂移核对 + 新场景 `refresh` 后人工审阅；clippy 单独跑并验退出码。
