# Contract v20：物品知识与未知名称投影

状态：协议 1.20 / contract-v20 active baseline

## 已建立边界

- 核心新增按物品种类保存的 `ItemKnowledge`。有 `appearanceNameKey` 且尚无知识记录的种类为 `unknown`；成功投掷后标记为 `tried`；`aware` 表示已经知道真实种类，并要求同时为 `tried`。
- `unknown` 与 `tried` 都只显示外观名称，`aware` 才显示内容定义的真实名称。当前原创浅色碎片使用“陌生的浅色碎片”作为外观名称。
- 背包、装备和地面物品 DTO 输出核心决定的 `displayNameKey` 与 `knowledge`。背包和装备中的 modifier、近战、射击及投掷 profile 在 `aware` 前不投影给 UI。
- HTML 只格式化 `displayNameKey`，不会从 `kindId` 重新选择真实名称。`kindId` 仍作为核心权威内容引用和稳定渲染标识存在；本地单机协议不是针对恶意客户端的内容保密边界。
- 当前唯一已有的通用物品使用动作是投掷，因此本切片只由成功投掷建立 `tried`。没有增加虚假的“鉴定”命令；`aware` 的实际发现入口由下一切片的消耗品效果提供。

## 协议、内容、存档与基线

- 协议升至 1.20，新增 `ItemKnowledgeDto`，物品投影新增 `displayNameKey` 和 `knowledge`。
- 原创内容包升至 1.15.0；物品可声明可选 `appearanceNameKey`，并禁止它与真实名称 key 相同。1.14.0 content hash 加入显式内置存档迁移白名单。
- save schema 继续为 v1，payload 新增可选 `itemKnowledge`。旧存档缺失该字段时按空知识表载入；载入时拒绝未知种类、重复记录、空记录以及 `aware` 但未 `tried` 的非法状态。
- 物品知识是新的权威规则状态，因此 state hash 升至 Schema v10，并按稳定种类 ID 顺序覆盖知识记录。
- contract-v20 从 v19 迁移 56 个 exact fixtures，并新增投掷未知物品后变为 `tried` 的场景，共 57 个。

## 后续

Stage D 的后续切片已由 [contract-v21](contract-v21-consumable-use-action.md) 建立内容驱动的消耗品与可观察 `aware`，并由 [contract-v22](contract-v22-instance-affix-knowledge.md) 建立实例级词条知识。随后扩展伪鉴定和完整鉴定；掉落表与程序化地牢仍排在物品知识闭环之后。
