# Phase 18: Outpost supply loop

状态：Gate 0–5 完成，Gate 6 UI 下一步。当前 active baseline 为 `contract-v161`；协议 `1.128`、save 容器 v1、state hash Schema `60`、demo 内容包 `1.151.0`（hash `6af8e97c7c2e4f1fa56b6c6d004d267cfb24d238f5921478740a45f5a567d478`）和 461 条 exact fixtures、零 waiver。金币、食物/饥饿、燃料/地牢光照、Outpost 以及 General Store 权威库存与交易已经建立，下一步只接玩家 UI 与完整流程验收。

## 1. 目标

Phase 18 建立第一个可持续重复的城镇补给循环：

```text
Outpost 购买补给
  -> 进入 Warrens
  -> 消耗食物与光源燃料
  -> 拾取地面或怪物掉落的金币
  -> 返回 Outpost
  -> 出售战利品或用金币再次补给
```

这一阶段不是先做一个空的城镇框架。每个新增状态都必须同时有来源、消费端、玩家可见反馈、存档/回放边界和正常 UI 操作。Outpost 首批只开放杂货店和 Warrens 入口；其他设施在具备实际服务时再进入地图。

## 2. 固定来源

行为参考固定为 RFB v1.3.0.7 本地源码 `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`。只提取行为事实与用户明确要求的少量物品数值，不复制旧地图字节、描述文本、算法实现或素材。

| 领域 | 固定来源 | Gate 0 结论 |
| --- | --- | --- |
| Outpost | `lib/edit/t_outp.txt`、`lib/help/town.txt` | 出生城镇；首批保留杂货店和 Warrens 入口的功能关系，地图独立设计 |
| 出生金币 | `src/py_birth.c` | 普通角色 `2d300+200`，上限 `999999999` |
| 地面金币 | `src/generate.c::_cave_gen_objects`、`src/object2.c::make_gold` | 普通层使用 `randnor(2,3)` 金币分配并按小地图缩放；金币价值由对象深度和币种基础值生成 |
| 怪物金币 | `src/xtra2.c::get_monster_drop` | 同时允许物品/金币的掉落有 20% 选择金币；Small kobold 的 `DROP_60` 因而不是恒定物品 |
| 食物 | `src/defines.h`、`src/dungeon.c`、`src/cmd6.c` | 保存饱食度；正常消化每 50 个能量脉冲；阈值驱动恢复、昏厥和挨饿伤害 |
| 出生补给 | `src/py_birth.c::py_birth_food/light` | Warrior 获得 5-9 份口粮、3-7 支火把；出生饱食度为 9999 |
| 光源 | `src/defines.h`、`src/dungeon.c::process_world_aux_light`、`src/cmd3.c` | 装备的普通光源每 10 个能量脉冲消耗 1 燃料；火把合并、灯笼使用油或另一盏灯补充 |
| 杂货店 | `src/shop.c` | 持久库存、店主、价格因子、买卖、每日维护；出生城镇额外保证一盏灯笼 |
| 物品数值 | `lib/edit/k_info.txt` | 首批只选择口粮、木制火把、黄铜灯笼和油瓶 |

项目的 `world_tick` 已经按调度器能量脉冲递增：速度 110 每脉冲获得 10 能量，标准 100 能量行动通常跨 10 个脉冲。因此原版的 10/50 脉冲周期直接落在 `world_tick`，不按 UI 命令数量或前端动画计时。

## 3. 首批内容选择清单

| 内容 | RFB 身份 | 数值 | 玩家作用 | 暂缓差异 |
| --- | --- | --- | --- | --- |
| Ration of Food | tval 80 / sval 35 | nutrition 5000，重量 1.0 磅，基础价值 3 | 恢复饱食度 | 不同时导入全部蘑菇、酒和特殊食物 |
| Wooden Torch | tval 39 / sval 0 | 容量 5000，重量 3.0 磅，基础价值 1 | 装备照明；消耗另一支火把补充 | 首批无光源 ego |
| Brass Lantern | tval 39 / sval 1 | 容量 15000，重量 5.0 磅，基础价值 30 | 装备照明；使用油补充 | 首批无神器灯和永久灯 |
| Flask of oil | tval 77 / sval 0 | 补充 7500，重量 1.0 磅，基础价值 3 | 给灯笼补充燃料 | 投掷油瓶与火焰地形效果暂缓 |

商店库存是经过明确筛选的兼容子集，不声称复刻原版杂货店的全部候选。为了让消费循环始终可用，首批杂货店在首次创建和每日维护后保证四类基本补给各有库存；随机数量、店主和价格仍使用确定性核心 RNG。原版 9-12 个不同库存槽只有在候选内容扩充后才恢复，不能用四个种类伪造九个不同商品。

## 4. 权威状态决定

### 4.1 金币

- 玩家保存 `gold: u32`，范围 `0..=999999999`。
- 地面金币使用独立 `GoldPile { id, position, amount, appearance }`，不是 `ItemInstance`，不占重量、背包槽或装备槽。
- `PickUp` 在同一格先按稳定实例 ID 吸收金币，再处理普通物品；金币达到上限时饱和，事件报告实际获得量。
- 金币堆随当前层和 `storedFloors` 保存。返回 Outpost 清除 reset-on-surface Warrens 实例时，未拾取金币与该实例一起清除。
- 购买、出售和拾取都是原子事务；非法目标、金币不足、库存不足或负重不足在状态/RNG/时间变化前拒绝。

### 4.2 饱食度

- 玩家保存 `nutrition: u16`，合法范围 `0..=15000`，出生值 `9999`。
- 阈值固定为 Bloated 15000、Full 10000、Hungry 2000、Weak 1000、Faint 500、Starving 100。
- 正常消化只在 `world_tick % 50 == 0` 结算，基础值使用当前速度对应的能量增益；首批 Warrior 没有快速/缓慢消化、德行或特殊姿态修正。
- `nutrition < 1000` 降低自然 HP 恢复；`< 500` 每个世界处理点有 10% 昏厥，成功施加 `1d4` paralysis；`< 100` 造成 `(100 - nutrition) / 10` 生命损失。
- 食用口粮是正常 `UseItem` 事务，消耗一件、增加 5000 并封顶 15000，随后支付正常使用行动成本。

### 4.3 燃料与可见性

- 内容定义声明光源容量或可提供的燃料；具体光源实例保存 `fuel.current/fuel.maximum`。
- 只有装备在 `light` 槽且需要燃料的光源才在 `world_tick % 10 == 0` 时消耗 1 点。
- 火把使用另一支火把补充 `source current + 5`，上限 5000；灯笼使用油瓶或另一盏灯的当前燃料补充，上限 15000。补充支付 50 能量。
- 燃料归零立即重新计算照明并发布结构化事件。Outpost 由环境光照亮；Warrens 不再拥有无条件玩家光源。
- 地图几何视线、照明、探索记忆和 actor 感知必须保持独立。地牢 terrain 的当前可见性依赖有光的 LOS；已探索但当前未照亮的格只显示记忆。首批 Human Warrior 没有额外红外规则，红外视觉后续单独扩展。

### 4.4 Outpost 与商店

- 内容格式增加 `towns` 和 `shops` 两个严格根；Gate 4 的 `TownDefinition` 引用稳定 floor ID 和 shop IDs，`ShopDefinition` 只声明城镇归属、类别和入口。店主、库存、价格与维护策略由 Gate 5 扩展。
- 现有 `demo.floor.surface` 保持稳定 ID，但由 `demo.town.outpost` 拥有并显示为 Outpost。旧存档不因重命名丢失返程目标。
- Outpost 使用固定、独立创作的室外地图；General Store 采用 `4×3` 实心墙体占地，仅保留入口格可行走。首批只绘制 General Store 和 Warrens 两个真实可交互入口，未实现的 2-9 号商店、家、博物馆和建筑不做成死门。
- 城镇楼层持久存在。返回 Outpost 仍按现有规则终止 Warrens 实例，但不清空店铺库存、店主或维护时间。
- 打开/关闭商店是零时间 UI 状态；`BuyFromShop` 和 `SellToShop` 是零世界时间但增加 revision 的权威命令。
- General Store 收购所有正价值且非遗骸的普通物品。店内物品完全识别；玩家卖出的物品保留完整实例状态并转为商店库存实例，回购时恢复同一 fuel、charges、affix、enchantment、curse 等状态。

## 5. RNG 与事件顺序

后续实现必须固定以下顺序：

1. 新建 Warrior 时先完成既有 build/HP 序列，再依次掷出生金币、口粮数量、火把数量和火把燃料，最后创建首次访问的杂货店。
2. 楼层生成继续先 terrain/connection、actor、普通物品，再生成金币、保证食物、保证光源和 guardian；每个被跳过的概率分支不抽内部对象 RNG。
3. Small kobold 先掷 `DROP_60`；成功后掷金币 20% 分支，只有所选分支继续生成金币价值或 Warrior 物品。遗骸保持在普通掉落之后。
4. 世界 tick 依次结算持续伤害、饱食/挨饿、自然恢复、装备燃料、设备恢复、怪物行动和能量获取；若玩家死亡立即中断后续阶段。
5. 交易先完整验证店铺可达性、库存、数量、金币和负重，再一次性移动库存与金币并发布事件；拒绝路径零 RNG、零时间、零 mutation。

任何为了现有 fixture 方便而改变上述顺序的实现都必须回到 Gate 0 重新审查。

## 6. Gate 顺序

### Gate 1: gold source and wallet (complete)

- 玩家金币、地面金币堆、拾取、Warrens 楼层金币和 Small kobold 金币分支；
- 协议、save、replay、state hash、事件、右栏金币和旧档零 RNG 迁移；
- 暂不加入商店，先证明金币来源本身完整。

### Gate 2: food and hunger (complete)

- 饱食度、出生口粮、食用、消化、恢复倍率、昏厥、挨饿死亡；
- Warrens 每层原版 50% 口粮保证尝试；
- 等待、休息、速度变化、死亡和存档续跑覆盖同一世界 tick 规则。

### Gate 3: fuel light (complete)

- 火把、灯笼、油瓶、实例燃料、出生火把和 refuel 命令；
- 装备燃料消耗、熄灭、Outpost 环境光和 Warrens 权威暗视野；
- Warrens 深度 1-9 每层原版 50% 光源保证尝试，其中 1/3 油、2/3 灯笼。

### Gate 4: Outpost content model (complete)

- `TownDefinition`、`ShopDefinition`、schema/编译/验证和持久 town/shop state；
- 扩展 `demo.floor.surface` 为独立设计的 Outpost，接入杂货店与 Warrens；
- 旧地表存档无 RNG 迁移到相同稳定 floor ID。

### Gate 5: General Store transactions (complete)

- 店主、基础补给库存、按 world tick 维护、原版相关价格因子、每单位收购报价 cap、批量买卖和完整识别；
- 交易守恒、负重、堆叠、出售后回购、每日维护和拒绝零 mutation 测试。

### Gate 6: player UI and acceptance

- 地图入口打开买入/出售双视图；右栏显示金币、饱食状态和装备光源燃料；
- 键盘/鼠标数量操作、确认、错误反馈和中英文 Fluent；
- 桌面 E2E 覆盖新建 Warrior -> Outpost 购物 -> Warrens 消耗 -> 获得金币 -> 回城补给 -> 保存/读取继续。

## 7. 版本与迁移边界

- Gate 0 不改运行时版本或 fixture；active baseline 保持 contract-v156。
- Gate 1 首次增加玩家金币、地面金币和存档字段，必须提升协议和 state hash Schema。旧档迁移为 0 金币，不补掷出生金币或旧楼层金币。
- Gate 2 旧档迁移到 9999 饱食度，不补发口粮、不推进世界时间。
- Gate 3 旧光源实例缺燃料时按其定义的生成默认值无 RNG 派生；普通非光源保持无 fuel。不得重新掷旧物品。
- Gate 4 旧 `demo.floor.surface` 缺少当前声明的 General Store 入口时，以明确、零 RNG 的 Outpost 布局重建；旧玩家坐标仍可行走时保留，否则回退到新出生点。当前和 stored surface 都可迁移，Warrens stored floors 不受影响。
- Gate 5 按当前开发期约束不兼容缺少商店状态的旧存档：新游戏在既有出生序列后初始化库存，读档本身不创建或刷新库存，缺失 `shopStates` 时严格拒绝。

Gate 1 的实际版本结果为协议 `1.124`、内容包 `1.147.0`、state hash Schema `56` 和 contract-v157。save 容器保持 v1；旧档钱包为 0、金币堆为空，既不补掷出生金币也不回填历史楼层金币。完整边界见 [Contract v157](contract-v157-gold-wallet.md)。

Gate 2 的实际版本结果为协议 `1.125`、内容包 `1.148.0`、state hash Schema `57` 和 contract-v158。旧档 nutrition 确定性迁移为 `9999`，不补发口粮、不推进 RNG。完整边界见 [Contract v158](contract-v158-food-hunger.md)。

Gate 3 的实际版本结果为协议 `1.126`、内容包 `1.149.0`、state hash Schema `58` 和 contract-v159。旧物品缺 fuel 时从内容默认值确定性派生，不重新生成物品或推进 RNG。完整边界见 [Contract v159](contract-v159-fuel-light.md)。

Gate 4 的实际版本结果为协议 `1.127`、内容包 `1.150.0`、state hash Schema `59` 和 contract-v160。旧地表仅在缺少声明入口时确定性重建；迁移保留合法玩家坐标，不推进时间或 RNG，也不重建当前/已存储 Warrens 楼层。完整边界见 [Contract v160](contract-v160-outpost-content.md)。

Gate 5 的实际版本结果为协议 `1.128`、内容包 `1.151.0`、state hash Schema `60` 和 contract-v161。店主、完整实例库存、维护时间、买卖命令与价格投影全部持久化；交易零世界时间，拒绝零 RNG/零 mutation。缺少商店状态的开发存档严格拒绝。完整边界见 [Contract v161](contract-v161-general-store-transactions.md)。

## 8. 明确暂缓

- Armoury、Weaponsmith、Temple、Alchemist、Magic Shop、Black Market、Home、Bookstore 和 Museum；
- White Horse Inn、城堡、Pest Control、其他任务与建筑服务；
- 荒野旅行、其他城镇、昼夜、镇民生成和城镇战斗；
- 全部普通食物、投掷油、光源 ego/神器、永久光源和特殊种族消化；
- Fame、virtue、no-selling、coffee-break 和特殊种族对价格/金币的修正；
- 低概率 `GREAT_OBJ` 金币/物品超深度提升，继续沿用 contract-v156 的暂缓决定。

这些项目不得以隐藏 no-op 字段或不可用入口提前进入生产内容。

## 9. Phase 验收

Phase 18 完成时，玩家必须能只通过正常 UI 在 Outpost 和 Warrens 之间重复完成补给循环；金币、饱食度、燃料、库存和未拾取金币必须在 core、save、replay、contract fixture、Tauri transport 和前端之间一致。验收关注资源循环与可用性，不增加纯平衡性质的完整通关矩阵。
