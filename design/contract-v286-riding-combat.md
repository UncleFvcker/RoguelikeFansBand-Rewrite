# Contract v286：骑乘战斗闭环

状态：已实现。Protocol `1.189`，State Hash Schema `v94`，内容包 `1.299.0`，save v1。

## 内容边界

- 本批不新增 item、ability、material、affix 或其他正式内容 ID。
- `ItemDefinition.ridingWeaponKind` 保存 RFB `master:lib/edit/k_info.txt` 的 `OF_RIDING`；
  当前包已有的 Sabre、Tulwar、Broad Sword、Long Sword、Falchion、Spear、Trident、
  Fauchard、Broad Spear、Glaive、Lance、Ball-and-Chain、War Hammer、Flail，以及固定神器
  Pain 共 15 件已登记。
- `lance` 是 RIDING 武器的专门语义：骑乘时 `+15` 命中、伤害骰数 `+2`。
- `ClassDefinition.ridingCombatExpert` 选择 Beastmaster/Cavalry 的固定骑乘惩罚分支；
  `mountedNonArrowBaseShotCap` 保存 Cavalry 非箭发射器的 `100` 上限。当前四职业保持默认值；
  后续 Cavalry 内容只需声明这两个字段，不需要修改核心。

## 规则

- 普通 `Ride` 只骑乘已由玩家控制的宠物；野生怪物不抽 RNG、不改阵营。“套马”仍归未来
  Cavalry 职业能力，本批不占用 ability ID。
- 快于 110 的坐骑按 `(mountSpeed - 110) * (riding*3 + level*160 - 10000) / 22000`
  计算受控速度并至少保持 110，再加 `(riding + level*160) / 3200`；较慢坐骑保留自身速度后
  加同一骑术增量。
- 普通骑手使用非 RIDING 武器或射击时，命中惩罚为
  `max(30, mountLevel - riding/80 + 30)`；骑乘战斗专家使用普通武器为 `-5`，箭为 `0`，
  其他弹药为 `-5`，弩栓翻倍为 `-10`。Cavalry 的非箭 `baseShot` 上限由职业字段承载。
- 非强制落马先按 contract-v285 公式增长骑术，再使用检定前 current 执行原版两阶段 RNG。
  强制落马跳过检定。落点按固定八方向做蓄水池抽样；无合法邻格时保持骑乘并承受
  `mountLevel + 3` 撞墙伤害，有落点时解除骑乘、承受同额坠落伤害并迁移玩家。
- 怪物近战每一击与怪物能力事务按实际玩家/坐骑伤害触发检定，伤害参数封顶 200；坐骑
  变形成不适合骑乘的形态时强制落马。坐骑死亡、宠物删除及召唤到期保持原版语义，只复用
  既有实体清理路径解除骑乘，不额外结算一次坠落。

## 版本与验收

新增字段只属于编译内容，事件继续投影为既有通用 `GameEventDto`，且无新增权威持久字段，
所以 Protocol 保持 `1.189`、State Hash Schema 保持 `v94`、save 保持 v1。内容包升至
`1.299.0`，active baseline 升至 `contract-v286`；现有 26 条 fixture 不进入骑乘路径，
验证零漂移，不刷新无关快照。
