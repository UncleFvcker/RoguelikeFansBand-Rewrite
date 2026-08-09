# Contract v236：荒野卷动城镇边界

状态：第六步已实现。协议 `1.160`，内容包 `1.226.0`，State Hash Schema `v79`，
active baseline `contract-v236`；save 容器保持 v1。

## 边界事务

- 普通荒野卷动先正规化 `wildernessPosition` 与 `wildernessViewOffset`。只有世界
  格坐标真正跨入 `WildernessLocationDefinition::Town` 时，才切换到该城镇已有的
  独立 FloorState；视口内暂时覆盖到城镇方向不会提前切换。
- 进入城镇后 `wildernessViewOffset` 归零，派生荒野地形缓存清空。该事务不调用
  荒野代推进，因此 `wildernessSeed` 保持不变；seed 仍只在进入世界地图时推进。
- 城镇切换不发送 `mapTranslation`，客户端直接接收独立城镇地图的全量格更新。

## 城镇状态

城镇地图、玩家落点、地面实体和物品继续保存在既有 FloorState；商店、Home 与
访问状态继续使用既有 town/shop/home 状态。首次进入仍按原路径生成城镇层，已经
访问过的城镇则从 `storedFloors` 恢复，本阶段不建立第二套持久化方式。

## 暂不包含

本阶段不把城镇建筑嵌入 96×33 荒野视口，也不实现从城镇地图边缘直接步行返回
连续荒野或城市郊区拼接；这些行为需要独立的设施坐标和边界设计。

## 验证

- 核心测试先通过既有世界图路径访问并保存第二城镇，再从相邻连续荒野卷动返回，
  锁定原地图、商店、Home 和访问状态恢复。
- 同一测试锁定缓存清空、视口偏移归零、seed 不变，以及进入城镇后的存档往返。
