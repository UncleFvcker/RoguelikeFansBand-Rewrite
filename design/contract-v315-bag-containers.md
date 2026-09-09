# E8.4：背包与现有容器系统

日期：2026-09-09；工作树 `RoguelikeFansBand-Rewrite-realms-items`，分支 `codex/realms-items`。
权威来源为 `D:/codex/Frogcomposband/master` 的 Git `master` 对象
`a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`；所有来源检查读取 Git 对象。

## 底材与生成

三种已有内容补齐原始身份和估值元数据。中文名逐项核对 `kind_name_zh.inc`；去除原版
量词、复数控制符后仍为下表名称，没有重新翻译。三者均为 `TV_QUIVER/SV_BAG = 46/1`，
由 source index 和 pval 区分；内容校验保留 source index 唯一约束，并允许不同 pval 的包
共享 tval/sval。原始重量均为 10（1 磅），原有分配深度和权重不变。

| source index | 底材 | pval | 普通容量 | Good 容量 | Holding 容量 |
| --- | --- | --- | --- | --- | --- |
| 722 | 布袋 | 0 | 4 | 6 | 8 |
| 723 | 皮囊 | 1 | 8 | 10 | 16 |
| 724 | 矮人背包 | 2 | 12 | 14 | 24 |

`noncraft.rs` 的容器入口按 sval 分支：背包以 `(pval + 1) * 4` 初始化，power 为 1 加 2，
power 大于 1 复用原版 Quiver Ego 权重选择与物化。包不抽箭袋的基础容量骰，不加 +20/+50。
四个 Ego 都保留源端可达性，包括在低于名义生成等级时以较低权重选中的结果。

生成结果写入既有属性包的 `bagCapacity`，表示最终非弹药格数；合并时替换容量，不相加。
移除三种内容、导入器、内容类型和 schema 中的 `inventorySlotBonus`。未随机生成的普通
作者配置物品由相同原始 pval 取得基础容量；已经生成的实例使用保存的最终值。
估值读取该最终容量，交由原版 Quiver COST_REAL 路径计算。

## 四种 Ego 的真实消费者

| Ego | 背包行为 | 源码依据 |
| --- | --- | --- |
| Holding（容纳之） | 容量翻倍 | `ego.c:obj_create_quiver` |
| Protection（保护之） | 不保护包内物品，也不保护另一个普通箭袋 | `spells3.c` 只检查 `SV_QUIVER` |
| Endless（原版名为无尽箭袋） | 保留激活；补充另外装备的箭袋，最多 50 发。没有箭袋时产量为零 | `devices.c:EFFECT_ENDLESS_QUIVER` 调用 `quiver_capacity/quiver_carry` |
| Phase（原版名为相位箭袋） | 包自身重量为零，内容重量照常计算 | `ego.c:obj_create_quiver` 与 `quiver.c:bag_weight` |

Phase 的携带重量判定现在同时核对 `46/0`，因此 Phase 背包不会让另一个普通箭袋内的弹药
减重。Protection 不获得名字暗示的额外保护，但原版估值加成保留；Endless 的激活估值同样保留。
不为两个原版全名 Ego 创造本地“背包版”中文名。

## 库存与持久化

原版分别存储 INV_PACK、INV_QUIVER、INV_BAG，并有显式转移动作。重写版保持现有统一
库存列表，自动将非弹药栈分配到背包格，箭袋继续按其既有数量规则分配弹药；没有添加独立
容器页面或持久化的第二份成员列表。本批等价范围为容量、物品类型边界与上述 Ego 消费者。

实际主库存占用 = 未进箭袋的物品栈数 − min（非弹药栈数，已装备背包容量）。只有该占用
不超过主库存基础容量时才允许提交。拾取、购买数量上限、可合并堆叠、换包、卸包、物品
变换拆栈、身体槽变化和读档校验使用同一分配规则。换包/卸包先检查投影，失败不移动或丢失
物品。库存总数仍投影实际物品格；详情只在容量已知时显示最终值，并说明不含弹药。
既有 Ego 名显示保留，不增加“完全鉴定”或“无 ego”提示。

Protocol 1.237、State Hash Schema v114、save header/payload v9、二进制容器 v1；包为
1.395.0，物品定义仍为 369。容量和 Phase 重量随实例保存，恢复不重新抽样，不添加旧开发
存档迁移。新增权威状态及共享投影，因此刷新并验证全部 26 条 active exact fixture，场景
输入不变、零 waiver。

## 验证

`generate-bag-reference.py` 抽取原版 `obj_create_quiver`、完整 Quiver Ego 选择及真实估值
函数，注入相同随机流，生成 972 组 C 对照：三个包、普通箭袋对照、power −1/0/1/2/3、
等级 20/50/80、随机与强制四 Ego，逐项比较容量、重量、Ego、RNG 次数和终态、COST_REAL。
这不表示重写版采用原版随机数发生器。

八组专项测试另行覆盖三底材 × 普通/Good/Great 的自然生成、四 Ego 源码可达结果、
未知/已知容量投影、满包合并、弹药边界、拒绝非法存档、换小包/卸包不丢物品、Phase 内容
重量、Protection 实际元素伤害、Endless 真实激活及保存恢复。UI 测试核对容量、Ego 名和
鉴定提示；前端 182 项测试、类型检查与 UI 构建通过。

| 检查 | 结果 |
| --- | --- |
| 核心单元测试 | 938 通过；其中旧哈希版本断言更新后单独复测，1 个既有全回放测试默认忽略 |
| 内容 / 本地化 / 协议 / 存档单元测试 | 134 / 39 / 7 / 2 通过 |
| workspace 检查与 Clippy（排除 rfb-tauri，all-targets，警告视为错误） | 通过 |
| 内容锁、协议及内容 schema | 通过 |
| active exact 基线 | 26 通过，场景输入不变、零 waiver |
| Ego 来源审计 | 160 身份、145 底材通过；本批补齐其中三个背包的来源元数据 |

不运行桌面 E2E，不生成本批桌面安装包，不把自动测试记作人工试玩。E8.5 随机神器和
后续职业/种族分支仍未完成。
