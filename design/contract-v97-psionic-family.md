# Contract v97：心灵族（psi 伤害 + 状态骑手组合）

状态：历史 baseline；当前 active baseline 见 [contract-v98](contract-v98-curse-family.md)。协议 1.97（唯一变更：`psi` 伤害类型加入 DTO 联合），内容包 1.88.0（content hash `bb07fafa930ab51316bb5f11c819dda81b3003b238dfa2bf5e7dbb4b161b9a1b`）；save 容器 v1；state hash 沿用 Schema v40。该 baseline 共 312 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

心灵族共 248 实例：MIND_BLAST 63、BRAIN_SMASH 123（含平坦覆盖 60/100/150/200）、PSY_SPEAR 62（多为显式骰 `1dX+150` 形）。原版语义（gf.c）：

- MIND_BLAST：豁免（dam/5 修正）门控骑手；失败 → 混乱 4+1d4 回合、几率幻觉、吸蓝 50；伤害恒定生效；
- BRAIN_SMASH：豁免（dam/3）门控全部；失败 → 伤害 + 致盲 8+1d8、混乱 4+1d4、麻痹 1d4（自由行动可挡）、减速 4+1d4 + 吸蓝 100；
- PSY_SPEAR：MST_BEAM 纯伤害射线，默认 1d(等级×3/2)+100。

新增伤害类型 **`psi`**（原版 GF_PSI 心灵能量，三枚举 + 转换 + web 文案「心灵异能」）。中性化简化（与 P33/P35 状态族先例一致，记录不复刻）：无豁免门（骑手恒定施加，psi 抗性缩短时长与减免伤害）、无吸蓝（DRAIN_MANA 族另迭代）、无幻觉（无对应状态）、固定时长（混乱/致盲 80 tick、麻痹 20 tick 防锁死、减速 80 tick——P35 世界 tick 约定）。

组合形态零新机制：MIND_BLAST/BRAIN_SMASH = 既有 `Sequence[Damage + ApplyStatus…]`（v85 echo-binding 同构，monster projectile Sequence 白名单早已放行，骑手 `resistanceType: psi` 走 v85 抗性缩时）；PSY_SPEAR = 既有 `beam-damage`（v78/v86 管线，**首个导入 beam**）。

## 2. 内容与演示（1.88.0）

demo 新增 `demo.actor.mind-lasher`（心灵鞭笞者，glyph "h"，自带 `{psi: resistant}` 抗性档呼应 v96）承载 `demo.ability.psi-lash`：Sequence[Damage psi 2d3+1，ApplyStatus 混乱 80t resistanceType psi]。

## 3. 契约场景（v97）

迁移 310 条（零语义漂移）后新增 311-312 同种子孪生对照：

- 311 心灵鞭击命中：psi 伤害 raw 7 → final 7（Normal），混乱 80 tick 施加；
- 312 玩家预置 psi 抗性（playerResistances precondition）：同种子同施法，final 4（减半）且混乱缩时 80→40——一条场景同证伤害减免与骑手缩时。

导入映射（mind-blast 双效果序、brain-smash 平坦编码 + 四骑手声明序、psy-spear 显式骰）由导入器单元测试覆盖。

## 4. 导入器映射（随后小步）

- MIND_BLAST → `Sequence[Damage psi 7d7, 混乱 80t]`；
- BRAIN_SMASH → `Sequence[Damage psi 12d12（平坦 N 经 1d1+(N-1)）, 致盲 80t, 混乱 80t, 麻痹 20t, 减速 80t]`（骑手全部 extend + resistanceType psi）；
- PSY_SPEAR → `beam-damage` psi，默认 1d(3L/2)+100、显式 `XdY+Z`；
- 实测收割 **248 实例全数**（casting 怪 829→831，法术映射累计 4097，未映射 1131）。

## 5. 验证

常规全套 + `migrate-baseline` 零漂移核对 + 新场景 `refresh` 人工审阅；clippy 单跑验退出码；五件套（协议 1.97 / pack 1.88.0 / content.lock / BUILT_IN+PREVIOUS / README）；本地桌面 E2E（contentVisualCount 76→77）。
