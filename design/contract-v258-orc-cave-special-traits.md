# Contract v258：Orc Cave 特殊怪物 trait

本批以权威 RFB `master` commit
`efd63661302866038f58d8cd2553b23e6af3bf9d` 为来源，完成 O5 的
`AURA_REVENGE`、`AURA_FEAR`、`TANUKI` 与 `UNIQUE2`。

## 行为边界

- `AURA_REVENGE` 在玩家接触攻击命中后按 `level / 150` 判定，成功时依次使用怪物
  的一项既有 blow；混乱或麻痹会禁用反击，反击本身不会递归触发反击。
- `AURA_FEAR` 在怪物首次进入视野、玩家完成接触攻击、或怪物受到非近战伤害时执行
  现有豁免与恐惧状态事务。非近战触发按距离削弱，同一 world tick 最多生效一次。
- `TANUKI` 出生时选定一次外观覆盖。投影与恐惧外观读取伪装定义，移动、战斗、抗性
  和死亡仍使用狸猫的真实定义；伪装随存档回环，不按行动重掷。
- `UNIQUE2` 与普通 Unique 共用“同一时刻最多一只”的分配和召唤资格，但死亡不写入
  永久击杀表，因此以后仍可出现。它不能成为目标变形、克隆式重复召唤、灭族或区域
  毁灭目标。当前项目没有捕获球运行时，不增加无消费者的捕获接口。
- `WILD_OCEAN` 只作为地点资格参与审计；海洋限定记录不进入 Orc Cave，纯分类旗标
  继续保留为 omission。

## 内容与兼容边界

Tanuki、Grendel、Fearmaster、Jade monk、Suke-san、Kaku-san 与 Silver Angel 进入
Orc Cave。审计为 201 imported、191 selected、7 blocked、28 excluded、1 guardian。

内容包升级到 1.249.0，共 618 actors、271 abilities 和 13 loot tables；严格同步
571 条，内容 hash 为
`1c7fdbf5023bd0ad898d8b828e024350f53d40c99a49b0bc7f8c58000fcffa2d`。协议保持
1.170，State Hash Schema 保持 v85，save 容器保持 v1。基线推进到
contract-v258，继续保留 22 个聚焦 exact fixture，不恢复旧 E2E。
