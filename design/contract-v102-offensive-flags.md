# Contract v102：装备旗标系统·进攻面（斩杀/击杀/品牌）

状态：已实现。协议 1.102 / 内容包 1.93.0（hash `66842fb3…`）/ state hash Schema v41（未变更）/ 326 个 exact fixtures、零 waiver。

## 1. 原版依据与内容表面

- 依据 FrogComposband `combat.c` 的 `slay_list`、`slay_tiers`、`tot_dam_aux` 与 `monster_living`。
- `ItemDefinition` / `AffixDefinition` 新增 `slays`（目标类别 → `slay|kill`）和 `brands`。
- slay 目标首版覆盖 animal/evil/good/living/human/undead/demon/orc/troll/giant/dragon；brand 覆盖 acid/electricity/fire/cold/poison。
- demo 新增屠龙刃（kill dragon）、余烬刃（brand fire）与寒霜猎手词条（slay animal + brand cold），并给 construct 纵切补 `nonliving` 标签。

## 2. 近战语义

- 仅玩家持武器近战消费进攻旗标；来源为所有已装备物品及其词条，符合原版装备可向武器旗标汇总的路径。
- 原版近战倍率按十分位整数保留：基础 slay 1.9×、中档 2.4×、高档 2.8×；对应 kill 为 4.0×、4.6×（原版整数截断）、5.6×。
- 五元素 brand 为中档 2.4×。只有目标对对应元素 `immune` 时该 brand 失效；resistant/strong 仍允许品牌倍率。
- 多个命中项只取最高倍率，不叠加、不相乘。倍率只放大武器骰，随后才加 `toDamage`，与 `tot_dam_aux` 的调用顺序一致。
- 匹配与倍率不抽 RNG；存档和 state hash 不新增字段。

## 3. 协议、知识与前端

- 协议 1.102：背包/装备 DTO 新增可选 `slays`/`brands`。
- 基础物品旗标由 kind awareness 门控，词条贡献由实例级已知 affix 门控；隐藏旗标照常参与权威战斗。
- Web 物品行以双语标签显示普通斩杀、强力击杀和元素品牌。

## 4. 导入回灌

- `SLAY_*` / `KILL_*` 折入 `slays`，强力 kill 覆盖同目标普通 slay；五种基础 `BRAND_*` 折入 `brands`。
- actor 导入补齐 11 类目标标签；living 按原版规则由 demon/undead/nonliving 的反集派生。
- 实跑结果：ego 107/160（原 105，+2）、神器 392/392；12 个词条/130 件物品带 slay，5 个词条/90 件物品带基础 brand。
- CHAOS/DARK/MANA/ORDER/VAMP/WILD 等特殊品牌继续留缺口；无装备槽/弹药形状上的少量普通旗标等待远程与投掷旗标管线。

## 5. 契约

- 323 条 v101 fixtures 迁移到 contract-v102；变化限协议版本、内容元数据和 DTO 扩展，既有玩法不获得新 demo 装备。
- fixtures 324–326 使用同 seed、同 1 点武器骰和固定 +1：kill dragon 得 6，fire brand 得 3，fire immune 回退到 2；三者均执行存档回读。

## 6. 遗留

- 弹药、发射器、投掷物的 slay/brand 组合。
- 混沌随机斩杀、吸血、魔力、秩序、狂野、黑暗等特殊品牌。
- 原版命中触发的自动旗标学习、元素易伤品牌翻倍、暴击/vorpal/on-hit 效果。
