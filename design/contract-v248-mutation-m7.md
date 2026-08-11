# Contract v248: mutation M7

状态：active baseline。协议保持 `1.166`，save 容器保持 v1，State Hash Schema
保持 v83。内容包为 `1.240.0`，content hash 为
`7676a7f483c522d0dc9bec9b633187a7382fd68eda3d1b82a599d35cf5c1a846`。

本批按 RFB `master` 的 source index 459 实现变形药水。它只调用既有随机变异
gain/lose、锁定保护、互斥移除和最终派生值刷新，不创建第二套选择器或持久状态。

- 未锁定变异多于一个时，先以 1/23 清除全部未锁定变异。
- 常规分支按原版 1/2 gain/lose、`1 / (6 - count)` loss 门槛及 1/2 连续循环。
- 零合法候选立即结束且零额外 RNG；一次药水事务最多刷新一次派生属性和资源。
- 权威中文名、青铜色 flavor、5,000 基础价值与两座城镇 Black Market 获取路径
  已进入正式内容，item adaptation 账本转为 active。

五个尚未闭环的随机候选仍由 `randomSelectionEnabled: false` 排除，变形药水不会
授予行为壳；104/104 候选闭环仍是 P3.7 发布门槛。变异账本保持 114 active / 38
blocked，随机候选保持 99/104 active。

Black Market 固定库存改变新游戏公共初始化 RNG 和物品实例分配，因此 21 个 exact
fixture 刷新到 `contract-v248`。原版循环、全清、锁定保护、5/6 阈值、连续改变、
互斥事件顺序、零候选、物品消耗和 RNG 次数由 Core 聚焦测试覆盖。
