# G2：单步自动拾取命令

状态：已实现。协议 `1.156`，state hash Schema 保持 `v76`，contract 基线 `v219`；save 容器保持 v1。

## 权威执行

- `AutoGet { objectId }` 只接受 Core 当前权威候选集中的对象；失效、过期或伪造 ID 零耗时结束。这样前端可锁定一个仍合格的目标，不受途中出现的新最佳目标干扰。
- 远端目标复用 `next_local_travel_direction`，转换为一次普通 `Move`，照常结算怪物、时间、饥饿、光源和地形。
- 目标在脚下时按实例 ID 处理：金币只收取指定金堆；`wanted` 执行该物品的墨家名器动作；`ammo` 拾取指定的 `=g` 物品。
- 脚下处理和普通 `PickUp` 不推进世界时间；命令序列、revision 和逻辑 turn 仍正常递增。

`MogaminatorDto.autoGetTarget` 同时投影对象 ID 与坐标，前端无需寻找同格对象或复制排序规则。

G2 不绑定 Ctrl+G，也不在单个命令内循环移动；连续派发及中断规则留给下一阶段。
