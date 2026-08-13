# Contract v287：骑兵与套马

状态：已实现。Protocol `1.190`，State Hash Schema `v94`，内容包 `1.300.0`，save v1。

## 正式内容与 ID

- 新增 `demo.class.cavalry`、`demo.skill-set.cavalry`、`demo.build.cavalry`、
  `demo.actor.cavalry-player`、`demo.ability.cavalry-rodeo` 与
  `demo.ability-program.cavalry-rodeo`。
- 不新增 item、material 或 affix ID。出生复用 `demo.item.broad-spear`、
  `demo.item.leather-scale-mail`、`demo.item.short-bow` 与 `demo.item.arrow`，箭数为 15–25。
- 属性 `+2/-2/-2/+2/+2/+1`，life 111%、base HP 10、经验 120%；八项技能基础为
  `20/18/32/1/16/10/60/66`，成长为 `10/7/10/0/0/0/22/26`，宠物维持除数 35。
- 骑术出生值/上限为 `2000/8000`；逐武器熟练度按 `master:lib/edit/s_info.txt` 的 N:22
  逐项映射。审计覆盖五个正式职业和当前 67 种基础武器。
- 中文职业名“骑兵”、能力名“套马”、职业与能力说明均使用 RFB `master:src/cavalry.c`。

## 套马规则

- 10 级职业能力，STR，消耗 0，基础失败率 50；沿用既有方向目标选择，目标必须是相邻、
  存活且可骑乘的 actor。已经骑乘时直接拒绝，不消耗能力检定 RNG。
- 先复用强制上马检定；目标原本就是宠物时只完成上马。野生目标再按原版计算：Unique
  等级先乘 3/2，高于 60 的部分再压缩；骑术、角色等级与目标有效等级先通过确定性门槛，
  再按原版短路顺序执行两次条件随机检定。
- `guardian` 与 `questor` tag 保留不可驯服身份。驯服成功后目标转为宠物并脱离原怪物包；
  失败发送甩落事件并进入 contract-v286 的强制落马事务。
- 普通 `Ride` 仍只接受已有宠物；套马不改变该共享边界。arena/battle 场景尚未作为正式
  世界状态存在，未来导入该状态时必须补回原版禁止驯服规则，不为本批增加占位状态。

## UI 与版本

- 新游戏增加骑兵构筑；三套 tileset 均提供骑兵 player actor 映射。角色面板复用骑术投影，
  套马复用既有职业能力列表和方向选择流程，不增加前端待处理状态或命令协议。
- `AbilityEffectSpecDto` 新增无参数 `rodeo` 投影，因此 Protocol 升至 1.190。没有新增权威
  持久字段，State Hash Schema 保持 v94，save 容器保持 v1。
- active baseline 升至 `contract-v287`。现有 26 条 fixture 不进入骑兵构筑或套马路径，
  全量 verify 零漂移，未刷新无关 fixture，零 waiver。
