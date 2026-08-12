# Contract v266：白马旅店“家里的麻烦”

状态：已实现。

## 权威来源

- `D:/codex/Frogcomposband/master` 的 `master` ref，核对提交
  `efd63661302866038f58d8cd2553b23e6af3bf9d`。
- `lib/edit/q_info.txt`、`lib/edit/q_inntrouble.txt` 与运行时中文表：quest 50 为
  `家里的麻烦 (前哨镇)`，危险等级 5，由白马旅店发布，目标是击杀 5 名
  `面相凶狠的雇佣兵`。
- 原版奖励按职业/种族取最后一个匹配项；当前正式玩家身份只有 Human/Warrior，
  因此本包可观察到的权威奖励是 `镶钉皮手套`。

## 地图与任务事务

- 白马旅店服务入口保持 Outpost `(63,13)`；独立任务入口使用 `(63,11)`。盗贼藏身处
  入口随其发布者迁到伯爵府北侧 `(30,9)`。
- 任务使用原版 38×17 固定地图。5 名雇佣兵是唯一任务目标；7 名
  `快乐唱歌的醉汉` 和一名按 `DEPTH+5 | NO_GROUP | NO_UNIQUE` 选择的随机怪物不计入
  完成条件。
- 固定物品使用 `itemSpawns`；勇气药水和烈酒在两个原版坐标间使用
  `scrambledItemPair`，每次楼层生成固定消耗一次二元乱序 RNG。三个 `$` 继续复用
  当前通用掉落表。
- 任务默认奖励保留原版硬镶钉皮甲，Warrior 覆盖为镶钉皮手套；单条奖励不消耗选择
  RNG。接取、进入、击杀计数、退出与领取继续复用既有任务状态机。

## 受限适配

- 原版醉汉的 `BEG` 是无效果近战表现；当前内容以空 melee routine 表达，不伪造伤害。
- `&` 使用编译内容中符合原版约束的 161 个 actor 候选快照，并沿用既有稀有度权重。
  `$` 使用 rewrite 的通用等权掉落表，不声称复刻完整 RFB 全局物品分配概率。
- 烈酒保留当前 Warrior 可观察到的混乱、幻觉、失忆与传送分支及 RNG 顺序。尚不存在
  的 Harmony virtue 与 Monk 醉拳系统不以占位状态模拟。
- 其余未激活职业/种族的奖励矩阵及缺失的 Protection affix 留待对应身份正式接入时
  实现；本批不增加无消费者的 race override 框架。

## 版本与验收

- 内容包 1.257.0；行为基线 contract-v266。
- Protocol 1.175、State Hash Schema v87 与 save v1 均不变。
- q50 接管 `(63,11)` 且盗贼入口迁至 `(30,9)`，改变了所有新游戏共有的 Outpost
  初始地形及状态哈希，因此统一刷新 21 条 active fixture；固定地图、任务目标、奖励
  和烈酒分支由 Content、Core 与 Localization 聚焦测试覆盖。
