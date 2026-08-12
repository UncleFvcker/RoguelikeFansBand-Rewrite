# Contract v262：Outpost 白马旅店

状态：已实现。权威来源为 `D:/codex/Frogcomposband/master` 的 `lib/edit/t_outp.txt`
与 `lib/edit/t_pref.txt`，均通过 Git `master` 对象读取。

## 范围

- 白马旅店使用原版中文名“白马旅店”和店主名“好人奥蒂克”，落在 Outpost
  `(63,13)`；它销售现有食物饮料，住宿价格为 20 金币。
- `innStayCost` 是可选商店内容字段；阿南巴旅店保持 25 金币，核心与 Web 不再硬编码
  某个旅店 ID 或固定价格。
- `TravelFromInn` 固定收费 500 金币，只接受已访问且拥有旅店的其他城镇。旅行直接
  到达目标旅店入口，不进入世界地图，不推进荒野 seed。
- 白马旅店入口同时投影 Quest Giver，盗贼巢穴任务由该服务发布；伯爵继续发布后续
  兽害任务。

## 暂缓

原版传闻、声望查询以及尚未导入的后续白马旅店任务没有权威消费者，本批不增加
no-op 命令、占位数据或通用建筑脚本。

## 兼容边界

- Protocol 1.173；State Hash Schema v86；save v1。
- 内容包 1.253.0；contract-v262；24 条 active exact fixture，零 waiver。
- 新增内容改变出生商店库存 RNG 与公共 Shop 投影，因此本批统一刷新 active fixture。
