# 弹药、发射器与竖琴 Ego 审计及 E4 实施计划

更新时间：2026-08-15

工作分支：`codex/items-next`

## 1. 审计结论

本次只通过 Git 对象读取 `D:/codex/Frogcomposband` 的 `master`；解析到的权威提交为
`efd63661302866038f58d8cd2553b23e6af3bf9d`。依据为：

- `master:lib/edit/e_info.txt`：16 条记录的 index、`T/W/C/F/E`；
- `master:src/ego_name_zh.inc`：逐 index 权威中文名；
- `master:src/ego.c`：发射器、弹药与竖琴的选择后实例化和 `ego_finalize`；
- `master:src/object2.c`：竖琴基础 pval、quality 调度、弹药骰面 super-charge 和 RNG 顺序；
- `master:src/cmd2.c`：Returning、Exploding、Endurance 的发射、命中和损毁时点；
- `master:src/combat.c`、`equip.c`、`types.h`：发射器倍率、射程和额外射击对攻击 profile 的影响。

结构计数如下：

| 项目 | 结果 |
| --- | ---: |
| `BOW` 记录 | 8 |
| `AMMO` 记录 | 6 |
| `HARP` 记录 | 2 |
| rarity 大于 0、可进入标准选择 | 16 |
| 有显式 `E:` activation | 0 |
| 有基础物品子类型限制 | 3 |
| 需要命中/消耗期消费者的弹药记录 | 3 |
| 已有正式定义但行为或身份不完整 | 2 |
| 现状可按权威行为直接开放 | 0 |

已有 `rfb-legacy.affix.endurance` 只闭合了部分毁坏免疫；`demo.affix.ammo-elemental` 是造箭使用的
临时候选，不带 source index、权威 rarity 或完整 Craft 弹药分支。两者都不能据此把本批判为已导入。

## 2. 16 条权威契约

`L` 为最低等级，`M` 为最高等级，`R` 为 rarity；`*` 表示无上限。中文名逐字来自
`ego_name_zh.inc`。本批 16 条均没有 `E:`，不得为它们增加 activation 或消耗 activation RNG。

| Index | 英文名 / 权威中文名 | 类型 | L/M/R | `C:` | 静态 flags 与选择后分支 |
| ---: | --- | --- | --- | --- | --- |
| 160 | `of Accuracy` / 精度之 | BOW | 0/40/1 | 10/5/0/0 | `AWARE`；分支先加 10 命中 |
| 161 | `of Velocity` / 极速之 | BOW | 0/60/1 | 5/5/0/0 | `AWARE`；倍率增加 `5+m_bonus(20)`，伤害先加 5 |
| 162 | `of Extra Might` / 额外力量之 | BOW | 20/*/2 | 2/4/0/3 | `STR`；倍率增加 `25+d25+m_bonus(50)` |
| 163 | `of Extra Shots` / 额外射击之 | BOW | 10/*/2 | 4/2/0/0 | `XTRA_SHOTS,AWARE`；pval 为 `1+m_bonus(4)` |
| 164 | `of Lothlorien` / 罗斯洛立安的 | BOW | 60/*/4 | 10/10/0/3 | `DEX,STEALTH,IGNORE_ACID/FIRE`；仅长弓；加倍率后 1/3 加额外射击，否则一项高抗 |
| 165 | `of the Haradrim` / 哈拉德人的 | BOW | 70/*/6 | 5/10/0/3 | `STR,IGNORE_ACID/FIRE`；仅重型弩；加倍率后 1/6 加额外射击并降低速度/潜行，否则一项高抗 |
| 166 | `of Buckland` / 雄鹿地的 | BOW | 30/*/3 | 10/5/0/3 | `SPEED,XTRA_SHOTS,IGNORE_ACID/FIRE`；仅投石索；1/3 加倍率，否则一项高抗 |
| 167 | `of the Hunter` / 猎人的 | BOW | 20/*/3 | 10/5/0/4 | `STEALTH`；1/5 强 ESP，否则弱 ESP，另有 1/30 动物 slay |
| 180 | `of Slaying` / 杀戮之 | AMMO | 10/*/2 | — | 调用共享 Slaying 的 `is_ammunition=true` 分支 |
| 181 | `(Elemental)` / `(元素的)` | AMMO | 20/*/3 | — | 四元素 `IGNORE`；调用共享 Craft 的 `is_ammunition=true` 分支 |
| 182 | `of Holy Might` / 神圣力量之 | AMMO | 60/*/7 | — | evil/demon/undead slay、火焰 brand、Blessed、四元素 `IGNORE` |
| 183 | `of Returning` / 返回之 | AMMO | 40/*/4 | — | 每次有效射击按 `50+level/2` 百分比决定是否不消耗弹药 |
| 184 | `of Endurance` / 耐力之 | AMMO | 40/*/4 | — | 四元素 `IGNORE`；普通碎裂、投影、魔法和怪物毁坏均免疫 |
| 185 | `of Exploding` / 爆炸之 | AMMO | 1/60/5 | — | 命中时以本次物理伤害作半径 3 `GF_MISSILE` 投射并必碎 |
| 195 | `of the Vanyar` / 凡雅精灵的 | HARP | 0/*/1 | — | `CHR,WIS,SUST_CHR,SUST_WIS,RES_DARK`，共用竖琴基础 pval |
| 196 | `of Erebor` / 伊鲁伯的 | HARP | 0/*/1 | — | `CHR,SUST_CHR,SUST_STR,SUST_CON,RES_FEAR,RES_BLIND`，共用竖琴基础 pval |

`AWARE` 在当前权威代码中没有 ego 运行时消费者，只保留审计事实，不为它发明效果。发射器的所有
动态附魔完成后，`to_d` 还要统一乘 `bow_energy(sval)/7150`；这一步不能提前到 `C:` 掷骰之前。

## 3. 当前可复用能力与真实缺口

可直接复用：

- E1 的 source-order、等级惩罚、rarity 选择核和 E2 的原子实例化入口；
- E3 的 `rfb_m_bonus`、Slaying/Craft、ESP、高抗、共享 pval、独立附魔和属性合并 helper；
- 弹药逐实例骰面、slay/brand 投射伤害、元素毁坏免疫与 Blessed 武器性质；
- Sniper 已有的爆炸投射事务，可参数化为普通 Exploding 弹药的固定半径 3；
- Archer 造箭的 quality、附魔和弹药 super-charge 流程，可改用本批完整六候选池。

必须闭合：

1. **基础物品身份。** 当前正式投石索、短弓、长弓、轻弩、重弩和 12 种弹药尚无
   `rfbBaseKind`。应锁定 k_info 160–164、175–178、185–189、190–192；还缺 source index 168 的
   `& Harp~` / `& 把~竖琴`。导入本批前共验证这 18 个 `(tval,sval)` 与 E3 基础物品不重号。
2. **发射器动态 profile。** `EquipmentBonuses` 尚不能表达逐实例倍率增量和额外射击。增加两个有
   明确单位的字段：倍率百分点增量，以及 RFB `base_shot` 百分制增量；`XTRA_SHOTS` 每 pval 增加
   15。最终倍率同时影响伤害和 `13+mult/80` 射程，额外射击在职业/Sniper/骑乘基础射速结算后、
   计算单次射击能量前加入。
3. **竖琴基础 pval。** RFB 在 ego 选择以前给每把 Harp 掷 `1+m_bonus(1)`；Bard 才把上限改成 2。
   该结果也驱动普通 Harp 的 CHR，不能只存在 ego 的 rolled state；基础 `NO_ENCHANT` 还必须阻止
   普通命中/伤害附魔。增加最小的实例
   `intrinsicProperties`，把基础 CHR 和 pval 一次物化并持久化；Vanyar 只补 WIS，Erebor 不重复叠加
   CHR。当前正式职业均走非 Bard 分支；Bard 导入时再接同一 helper，不为 E4 添加假职业定义。
4. **弹药行为。** Returning 和 Exploding 是固定内容性质，不需要新增逐实例随机状态。给 affix 定义
   增加最窄的 typed ammunition behavior；Endurance 继续复用现有毁坏免疫字段。消费者不得直接按
   本地化名称判断。
5. **堆叠与时点。** 一整堆弹药只选择一次 ego、物化一次随机结果；拆分、发射和读档复制已有结果，
   不重掷。Returning 在合法目标确认后、拆栈与实例 ID 分配前掷骰；成功仍完成攻击，但不减数量。
   Exploding 只在命中后使用已算出的 `tdam`，且普通 breakage 优先级保持“强制模式 → Endurance →
   Exploding → 普通弹药”。

本批没有 activation，因此不增加 effect program、charges 或 activation DTO。Holy Might 的 slay、brand、
Blessed 与 `IGNORE` 全部由现有静态属性表达；不得为它增加专用战斗分支。

## 4. 实施顺序与提交边界

所有子批独立提交。定义可以先存在于测试输入，但只有 E4.7 完成后才进入自然生成。按用户约定，
本工作树只跑每个子批新增的测试和必要的内容/schema 校验；完整测试留给合并工作树。

### E4.1：锁定 16 条权威契约与 18 个基础身份

状态：已完成。新增 ranged expectation 表，逐条校验 8 BOW、6 AMMO、2 HARP 的权威中英文名、
`T/W/C/F/E`、三个子类型限制和动态分支；同步命令从 RFB `master` 回灌 5 个发射器与 12 种弹药的
`rfbBaseKind`，并导入 source 168 Harp 的权威基础定义与中英文名。18 个 `(sourceIndex,tval,sval)`
通过正式内容测试锁定，Harp 保持 launcher 槽、`NO_ENCHANT` 和无 projectile profile；生成期 CHR/pval
仍留给 E4.2/E4.4。内容包推进至 `1.379.0`；本批没有新增 schema、Protocol、save 或 State Hash 字段。

- 增加最小 expectation 表，逐条锁定 index、权威中英文名、类型、L/M/R、`C/F/E`、子类型限制和
  动态分支；不建设 `ego.c` C 源码解析器；
- 为 5 个发射器、12 种弹药回灌 `rfbBaseKind`，导入 source index 168 Harp 的权威基础定义和中文名；
- 验证 18 个基础物品 `(tval,sval)` 唯一，并锁定 Lothlorien/Haradrim/Buckland 的子类型对应关系；
- 16 条都使用既有 automatic 名称组合；为 `(...)` 与 `of ...` 形式增加中英文组合测试。

提交目标：`feat: add ranged ego source identity`

### E4.2：扩展发射器与竖琴物化状态

状态：已完成。`EquipmentBonuses`/DTO 增加倍率百分点与 `base_shot` 百分制增量；物品实例及四类
save DTO 增加 `intrinsicProperties`，并接入装备聚合、可见物品投影、堆叠判定与 State Hash。
`EgoMaterialization::apply_to` 仅在完整物化提供基础属性时一次提交，普通 ego 应用不会清空既有
基础属性。Protocol 推进至 `1.228`、save header/payload 推进至 v5、State Hash Schema 推进至
v108；未刷新或执行全量 fixtures。

- 给 `EquipmentBonuses` 增加 launcher multiplier delta 和 base-shot delta 两个有单位字段；
- 给物品实例增加 `intrinsicProperties`，仅保存 Harp 这类基础物品在生成期掷出的真实属性；
- 让两类状态进入装备聚合、物品投影、save 和 state hash，读档不得重掷；
- `EgoMaterialization::apply_to` 仍一次提交完整附魔、properties 与实例状态，失败/重试不留下半成品；
- 从当前基线一次性推进 Protocol `1.228`、save header/payload v5 与 State Hash Schema v108；若执行时
  主线基线已前进，则分别只推进一个版本。后续 E4 内容提交不重复推进。

提交目标：`feat: add ranged ego materialization state`

### E4.3：实现 8 条发射器 source-index 分支

状态：已完成。160–163/167 共用发射器原子物化、独立 `C:`、共享 pval 与强/弱 ESP helper；
164–166 在任何分支 RNG 前校验长弓、重弩和投石索，不相容选择保留选择 RNG 后重试。倍率增量先按
`bow_energy/10000` 换算，最终伤害附魔按 `bow_energy/7150` 缩放；最终倍率统一进入伤害与
`min(18,13+mult/80)` 射程，额外射击在职业、Sniper 和骑乘基础射速之后进入能量计算。Hunter
所需的永久 Telepathy/Nonliving ESP 令 Protocol 推进至 `1.229`，save 与 State Hash schema 保持不变。

- 按原顺序实现 160–167 的分支、独立 `C:` 掷骰、共享 pval、倍率能量换算与最终 `to_d` rescale；
- Lothlorien 仅长弓、Haradrim 仅重弩、Buckland 仅投石索；拒绝后重新选择并保留额外 RNG 消耗；
- Hunter 复用 E3 强/弱 ESP helper，Lothlorien/Haradrim/Buckland 复用高抗 helper；
- profile 使用最终倍率重算伤害与射程，在已有职业/Sniper/骑乘射速后叠加额外射击；
- 固定 seed 覆盖 Accuracy、Velocity、Extra Might、Extra Shots、Hunter，以及三个子类型成功/拒绝分支。

提交目标可拆为：

1. `feat: materialize basic launcher egos`
2. `feat: materialize restricted launcher egos`

### E4.4：实现普通竖琴与 2 条 HARP ego

状态：已完成。普通 Harp 在固定神器判定后、quality/ego 调度前执行一次非 Bard
`1+m_bonus(1)`，将 CHR 保存到 `intrinsicProperties`；195/196 从该实例属性复用 pval，分别补齐
Vanyar 的 WIS/sustain/暗抗与 Erebor 的 sustain/恐惧及盲目免疫，不重复 CHR 或消耗 pval RNG。
HARP 使用独立选择池，基础 `NO_ENCHANT`、launcher 槽和无 `projectileProfile` 均保持不变；新增测试
覆盖普通竖琴的装备/卸下聚合、Vanyar/Erebor、精确 RNG 次数和 save round-trip。

- 在 quality/ego 调度前物化普通 Harp 的 `1+m_bonus(1)`，写入 `intrinsicProperties` 的 CHR，并保持
  基础 `NO_ENCHANT`，不生成无意义的命中/伤害附魔；
- 195/196 只使用同一个基础 pval 添加其余属性、sustain 与抗性，不能再次掷 pval 或重复 CHR；
- Harp 占用 launcher 装备位，但没有 `projectileProfile`，不能用于发射弹药或获得弓箭射程；
- 覆盖 ordinary、Vanyar、Erebor、save round-trip 和装备/卸下属性聚合。

提交目标：`feat: materialize harp egos`

### E4.5：实现 6 条弹药物化与完整造箭候选池

状态：已完成。180/181 复用共享 Slaying/Craft helper，182 的 Blessed 写入实例性质，183/185 使用
`AmmunitionBehaviorDefinition`，六条统一在动态分支后执行骰面 super-charge。Archer 已删除私有二候选
近似，改走共享 source-order selector/materializer；固定 seed 精确锁定一次选择、动态物化、强化及 RNG
顺序，并验证拆堆和读档复制已有结果。为使 Archer 当前即可使用完整池，六条 AMMO affix 已先进入正式
内容，source 181 临时定义也已替换；自然生成 policy 与其余十条 ranged affix 仍留给 E4.7。内容包推进至
`1.380.0`，只增加 content schema 的 typed behavior，未推进 Protocol、save 或 State Hash schema。

- 180 复用 `roll_rfb_slaying(..., true)`，181 复用 `roll_rfb_craft(..., true)`；
- 182/184 写入既有静态 slay、brand、Blessed 和毁坏免疫，183/185 写入 typed ammunition behavior；
- 所有 AMMO ego 物化后执行共同骰面 super-charge：触发率 `1/(5+200/max(level,1))`，连续增 `dd`，
  最终上限 9；
- Archer 保留 quality、附魔和前置 RNG 时点，但把私有 Slaying/Elemental 二候选替换成共享六候选选择器；
- 堆叠固定 seed 测试断言一次选择、一次动态物化、一次 super-charge，拆分和读档不重掷。

提交目标：`feat: materialize ammunition egos`

### E4.6：闭合 Returning、Exploding 与 Endurance 消费者

状态：已完成。射击 profile 现在携带 typed ammunition behavior 与 source 184 身份；Returning 在目标
与路径验证后、拆栈前逐发判定，成功时完整执行射击但不创建弹药实例。Exploding 在普通命中伤害掷出后
复用既有物理爆炸事务，以相同 `tdam` 施加半径 3 投射；Sniper Exploding 分支先行。破损顺序固定为
强制模式、Endurance、Exploding、普通概率；Endurance 的元素、魔法、地面投影和怪物毁坏继续复用已有
静态属性消费者。三个固定 seed 测试覆盖合法性、命中/miss、优先级、RNG、数量与实例 ID 时点。

- Returning：合法射击才掷返回概率；成功路径照常进行命中判定与结算，但不拆栈、不分配新 ID、
  不生成地面弹药；失败路径只在既有时点拆出一发，取消和非法目标不消耗 RNG；
- Exploding：命中后以同一个 `tdam` 执行半径 3 物理投射，并走必碎；miss 不爆炸，Sniper 专属爆炸
  模式保持更高优先级；
- Endurance：保持普通 breakage 为 0，并覆盖元素/魔法/怪物毁坏；强制破坏射击模式仍可覆盖它；
- 每个行为覆盖成功、条件不满足、优先级和 RNG/数量/实例 ID 不应消耗的路径。

提交目标：`feat: implement ammunition ego consumers`

### E4.7：导入内容并开放自然生成

- importer 生成并核对 16 条正式 affix 和 16 组权威中英文消息；E4.5 已落地的六条 AMMO 定义必须与
  importer 输出一致，英文重名的 Ammo Slaying 继续使用 source index/type 消歧，不能覆盖 WEAPON source 1；
- E4.5 已保留 `rfb-legacy.affix.endurance` 稳定 ID、补齐 source 184 元数据，并用正式 source 181
  删除了 `demo.affix.ammo-elemental`；本批只需验证 importer 可重复生成相同结果；
- 把 `base-items` 的窄 RFB ego policy 从 `WEAPON/DIGGER` 扩到本批完成的 `BOW/AMMO/HARP`，其余
  护甲仍走旧 policy，直到 E5；
- 自然生成的 incompatible launcher ego 必须按权威重试；Harp 不进入 BOW 池，BOW 也不进入 HARP 池；
- 更新 pack version、content lock 和只受本批影响的 fixtures；纯内容提交不推进 Protocol、save 或
  State Hash schema。

提交目标：`feat: import launcher ammunition and harp egos`

## 5. 聚焦验证矩阵

本分支不跑完整测试。每个子批只运行刚新增测试的精确名称，并按真实变化追加最小校验：

```powershell
cargo test -p rfb-legacy-import <本批新增测试名>
cargo test -p rfb-content <本批新增测试名>
cargo test -p rfb-core <本批新增测试名>
cargo run -q -p rfb-content --bin rfb-contentc -- inspect-source packs/rfb-demo-original
cargo run -q -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo fmt --check
git diff --check
```

E4.2 额外生成并核对 Protocol bindings、content/protocol schema，且只跑新增的 save/state-hash round-trip
测试；不执行 `verify-all`、`refresh-all` 或完整 replay。E4.7 只刷新本批改变的内容锁和行为 fixture。

## 6. 完成门禁

E4 只有同时满足以下条件才完成：

- [ ] audit 恰好得到 8 BOW、6 AMMO、2 HARP，16 条全有权威中文名和非零 rarity；
- [ ] 18 个基础物品有唯一 `rfbBaseKind`，普通 Harp 及其非 Bard pval 可自然生成并稳定读档；
- [ ] 三个发射器子类型限制严格重试，所有倍率、伤害附魔、射程和额外射击使用最终实例结果；
- [x] 六种弹药共享一个 selector/materializer，Archer 不再维护二候选近似池；
- [ ] 整堆弹药只物化一次，拆分、发射和读档不重掷；
- [ ] Returning、Exploding、Endurance 都有真实消费者和顺序测试，不存在仅显示名称的 no-op；
- [ ] 16 条无 activation 的事实被锁定，没有虚构 effect 或无关 DTO；
- [ ] 自然生成仅扩到 `BOW/AMMO/HARP`，E5 前不开放护甲 ego；
- [ ] 每个子批只运行新增测试并单独提交，完整测试明确留给合并工作树。
