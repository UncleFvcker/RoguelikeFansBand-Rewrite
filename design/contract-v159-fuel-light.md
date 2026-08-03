<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v159: Fuel and dungeon light

状态：historical baseline；Phase 18 Gate 3 完成。协议升至 `1.126`，save 容器保持 v1，state hash Schema 升至 `58`。demo 内容包升至 `1.149.0`，content hash 为 `8d1abd50ee3b9b849d58ae3cd0dddc3dbe4b28963b94bc6d642c7c33828a26e1`；该基线包含 459 条 exact fixtures、零 waiver。

## 固定来源结论

固定 RFB v1.3.0.7 来源中，Warrior 出生获得 `3-7` 支木制火把，每支初始燃料为 `1500-3500`、步长 500。普通光源每 10 个能量脉冲消耗 1 fuel。火把使用另一支火把合并并额外增加 5 fuel；黄铜灯笼使用油瓶或另一盏灯的剩余 fuel，补充动作支付 50 能量。

## 内容与实例

- `ItemDefinition.fuel` 严格声明 `kind`、`initial`、`maximum` 和 `lightRadius`。`torch`、`lantern`、`oil` 是当前唯一合法种类；内容验证拒绝零容量、初始值超容量、不可装备却带照明半径以及不一致的光源形状。
- Wooden Torch 重 3.0 磅，默认 `4000/5000` fuel、半径 1；Brass Lantern 重 5.0 磅，默认 `7500/15000`、半径 2；Flask of oil 重 1.0 磅，提供 `7500/7500`、半径 0。
- 每个具体物品实例保存自己的 `fuel { kind, current, maximum, lightRadius }`。火把和灯笼不堆叠；同一实例内的油瓶堆共享内容定义给出的单瓶供油量，补充时只消耗一瓶。
- 生产 Warrior 在金币和口粮 RNG 之后抽取火把数量与共同初始燃料；历史 build 不获得该出生补给，也不增加 RNG。
- Warrens 深度 1-9 每层在口粮阶段后执行一次 50% 光源保证尝试；成功时按 1:2 权重生成油瓶或黄铜灯笼，落点和实例随楼层保存。

## 行动与世界处理

- 协议新增 `RefuelLight { targetItemId, sourceItemId }`。目标必须装备在 `light` 槽、尚未充满，来源必须是背包中另一件有燃料且兼容的物品。
- 火把来源贡献 `source current + 5`；灯笼接受油或另一盏灯并贡献来源 current。补充后封顶且消耗一个来源实例，成功支付 50 能量并发布 `light.refueled`。
- 缺失目标/来源、同一实例、错误位置、满燃料、空来源或类型不兼容在时间、RNG 和物品变化前拒绝，并发布带原因的 unavailable 事件。
- 每逢 `world_tick % 10 == 0`，装备且有照明半径的燃料光源消耗 1；降到零时立即发布 `light.extinguished` 并失去光照。
- Outpost 地表保持环境光。Warrens 取消无条件玩家光源：几何 LOS 内仍只有装备有效火把的半径 1 或灯笼的半径 2 格当前可见；已探索但未照亮的地形只显示记忆。

## 迁移与确定性

- save 容器仍为 v1。旧实例缺 fuel 时按当前 item content 的 `initial/maximum/lightRadius` 确定性补齐；普通非燃料物品仍为 `None`。
- 迁移不重新掷出生火把或楼层补给，不改变 revision、turn、world tick、RNG 或物品身份。
- state hash Schema 58 纳入实例 fuel。协议 1.126 的命令、物品 DTO、事件、TypeScript bindings 和 JSON Schema 同步更新。

## UI 与暂缓边界

- 背包与装备列表显示当前/最大燃料。未充满的已装备光源提供补充按钮，并只在背包存在兼容来源时可用；中英文事件覆盖成功、拒绝和熄灭。
- 本 Gate 不加入投掷油、光源 ego/神器、永久光、混色、地形发光、红外视觉或商店补给；这些规则不以隐藏字段进入生产内容。

## 验收

- 核心测试覆盖 Warrior 出生数量/燃料/RNG 顺序、三种燃料内容、火把与灯笼补充、50 能量成本、拒绝零 mutation、周期消耗、熄灭、存档迁移、地表/地牢可见性和 Warrens 生成权重。
- fixture 459 固定装备 `1000` fuel 火把后使用 `2000` fuel 火把补充：装备阶段先消耗 1，最终 fuel `3004`、补充量 `2005`、world tick `15`，并完成 save round-trip。全部 459 条 active fixtures 保持 exact，零 waiver。
- 内容源码验证、协议/schema 生成检查、Rust workspace、contract、Web 测试、类型检查与 UI 构建共同构成 Gate 3 验收。
