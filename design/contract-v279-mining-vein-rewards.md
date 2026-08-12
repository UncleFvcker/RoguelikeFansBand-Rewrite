# Contract v279：矿脉、金币与材料收益

状态：active baseline

## 权威边界

- 岩浆矿脉按 `1/60` 生成已知富矿；未命中后按 `1/20` 生成隐藏富矿。
- 石英矿脉按 `1/30` 生成已知富矿；未命中后按 `1/10` 生成隐藏富矿。
- 隐藏富矿使用既有 `concealedAsTerrainId` 与搜索系统；即使未被搜索发现，直接挖除仍按
  真实富矿结算。
- 四个新增 terrain ID 为 `demo.terrain.magma-hidden-treasure`、
  `demo.terrain.quartz-hidden-treasure`、`demo.terrain.magma-treasure` 与
  `demo.terrain.quartz-treasure`。本批不新增 item、material、ability 或 affix ID。

## 玩家挖掘事务

成功移除矿脉时严格按以下顺序结算：

1. 地形替换为 `digging.resultTerrainId`。
2. 按 contract-v278 增加挖矿熟练度。
3. 使用增长后的熟练度生成材料。
4. 富矿生成金币。
5. 判定额外物品。

材料数量基数为 `1 + depth / 25 + mining / 1600`。富矿必给铁矿石；20 层起 `1/3`
给银矿砂，40 层起判定秘银粉尘。普通矿脉 `2/3` 给半量铁矿石；30 层起 `1/6` 给
水晶碎片。所有除法均为整数除法。

富矿金币生成等级为
`max(objectLevel, depth) + min(20, mining / 500) + depth / 10`，限制为 1–100；金额再乘
`100 + min(200, depth * 2) + mining / 80`%。当前 rewrite 没有独立的楼层 object level，
因此以 depth 作为该输入。金额超过 32767 时按原版减去一次 `randint0(1000)`；可见掉落
发送 `terrain.found-something`。

## 其他地形来源

- 魔法摧毁富矿只生成普通金币，不增加熟练度，不生成材料或额外物品。
- 怪物破墙不获得玩家挖矿收益。
- 碎石继续使用 soft power 10；原版随机物品掉落复用楼层 loot 管线，来源投影为
  `ItemOriginKindDto::Rubble`。
- 富矿额外物品复用既有楼层 loot 与普通/优秀/极好质量。原版 artifact 尝试、幸运、
  德行和特殊模式修正仍是共享物品生成缺口，不在挖矿代码中伪造。

## 协调版本与验收

- Protocol 1.181；demo pack 1.277.0；content hash
  `9e84e738fecbc3b74933c4a708c5a89cd77dd7bdd000c11b76c7d57184abec26`。
- State Hash Schema 保持 v90；save 容器保持 v1。
- active baseline 为 contract-v279，共 26 条 exact fixture、零 waiver。场景 484 固定验证
  隐藏富矿在未搜索状态下被挖除后依次获得成长、材料、金币、额外物品并通过存档回放。
