# Contract v18：重量射程与投掷攻击

状态：协议 1.18 / contract-v18 active baseline

## 已建立边界

- 物品定义新增整数 `weightTenthsPound`，所有原创物品均明确声明重量；可造成投掷伤害的物品另行声明 `throwProfile`，包含命中修正、伤害修正、伤害骰和类型。
- 投掷射程使用纯整数公式 `clamp(50 / weightTenthsPound, 2, 10)`。常量 50 是当前基础投掷力；力量属性和职业修正建立后可替换该输入，不改变重量单位。
- 投掷使用独立 `ThrowHit` 检定。基础投掷技能暂由玩家攻击 rating 派生，叠加物品 `toHit`，眩晕通过既有派生属性管线削弱该技能。
- 命中后复用 `DamagePacket`、物理护甲、元素抗性和死亡移除；输出结构化 `combat.throw-miss`、`combat.throw-hit` 与 `combat.throw-slay` 事件及 projectile trace。
- 投掷事务在 RNG 与伤害之前先完成单件提取和必要的稳定 ID 分配；无论命中或失误，物品均落在权威 `landing`。返回武器、药水破裂与特殊折损仍不在本阶段。

## 协议、内容与基线

- 协议升至 1.18，新增 `ThrowProfileDto`，背包/装备 DTO 输出单件重量和可选投掷 profile。
- 原创内容包升至 1.13.0；1.12.0 content hash 加入显式内置存档迁移白名单。
- save schema v1 / state hash schema v9 不变；重量/profile 是内容定义，投掷结果继续由既有角色、物品、RNG 和分配器状态覆盖。
- contract-v18 从 v17 迁移 54 个 exact fixtures，并新增一条投掷命中 fixture，共 55 个。

## 后续

阶段 C 的普通近战、射击和投掷纵向闭环已经建立。下一阶段优先利用现有整数重量建立背包总重量与容量，再进入鉴定/掉落；投掷目标模式、返回武器、药水破裂和动画按各自系统单独扩展。
