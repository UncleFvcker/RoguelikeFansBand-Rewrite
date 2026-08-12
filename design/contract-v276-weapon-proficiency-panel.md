# Contract v276：逐武器熟练度角色面板

## 投影

- `PlayerProgressDto.weaponProficiencies` 直接读取 contract-v275 的权威职业表和稀疏成长
  状态，不保存第二份进度。
- 每项包含规范基础物品 ID/name key、近战或发射器分类、当前值、职业上限、原版等级和
  原版命中加成。神器与特殊变体通过 `weaponProficiencyBaseItemId` 共享基础行，不重复投影。
- 等级边界为 `< 4000` 生疏、`< 6000` 入门、`< 7000` 熟练、`< 8000` 专家、其余大师。
  近战、投石索与弓显示 `(current - 4000) / 200`，弩显示 `current / 400`。

## 界面

- 角色成长面板增加默认折叠的“武器熟练度”区域，按近战武器和发射器分组。
- 英文等级使用 `Unskilled / Beginner / Skilled / Expert / Master`；中文使用
  “生疏 / 入门 / 熟练 / 专家 / 大师”。
- 前端只负责分组、本地化和格式化，不重新计算等级、上限、别名或命中加成。

## 契约与版本

- 原 21 条 active fixture 因共享玩家投影变化全部刷新；另增加近战成长、射击成长和
  熟练度存档回放 3 条聚焦契约，active 集共 24 条、零 waiver。
- Protocol 为 1.179。1.178 已用于 contract-v275 的必填存档 DTO，因此本次快照 DTO
  独立推进一个协议版本。
- State Hash Schema 保持 v89，save 容器保持 v1，内容包保持 1.275.0；没有新增任何
  item、ability 或 affix ID。
