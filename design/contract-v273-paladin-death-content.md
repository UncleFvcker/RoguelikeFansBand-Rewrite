# Contract v273：死亡领域圣骑士正式内容

状态：已实现。

## 权威来源

- `D:/codex/Frogcomposband` 的 `master` ref，核对提交
  `efd63661302866038f58d8cd2553b23e6af3bf9d`。
- `src/paladin.c`：职业名称与说明、六项属性修正、生命/经验/基础 HP、八项技能、
  WIS 施法、最低失败率、负重和出生装备。
- `lib/edit/m_info.txt` 的 Paladin / Death 段：四册 32 条死亡祈祷的等级、法力消耗
  和基础失败率。

## 正式内容

- 新增 `demo.class.paladin`、`demo.skill-set.paladin`、
  `demo.build.paladin-death` 和 `demo.actor.paladin-player`。
- 属性修正为 `+2/-3/+1/0/+2/+2`；生命 110%、经验 135%、基础 HP 12；八项
  基础/成长技能逐项使用原版 Paladin 表。
- 施法使用 WIS、`rfb-mana`、职业最低失败率 5%，负重为 `450/20/1200`；死亡
  build 使用单领域学习容量公式和 `divine-random` 神授模式。
- 出生直接引用既有 `demo.item.broad-sword`、`demo.item.ring-mail` 和
  `demo.item.black-prayers`，没有新增 item 或 ability ID。

## 投影与界面

- 新游戏列表开放“圣骑士 · 死亡”，职业名仍为原版“圣骑士”，领域单独显示。
- 三套 tileset 为 `demo.actor.paladin-player` 提供稳定外观映射。
- 四册死亡祈祷继续使用通用按书分组 UI；`divine-random` 复用 contract-v272 的
  书本级“学习祈祷”入口，玩家不能点选具体祈祷。

## 版本与验收

- 内容包升至 1.273.0，content hash 为
  `132b2a15ebcd5b74e2949817b45c88e576c0fae37eaa2e72548972249d70e1ae`。
- Protocol 1.177、State Hash Schema v88 与 save v1 不变；新增 build 不改变既有
  fixture 的默认 Warrior 初始化。active baseline 升至 contract-v273，21 条 exact
  fixture 复验零漂移，保持零 waiver。
- 定向测试覆盖职业身份、六项修正、八项技能、出生装备、WIS 法力、神授学习投影和
  32 条死亡祈祷表的首尾与等级边界。

## 明确不在本批

- 30 级“地狱长枪”和 40 级恐惧抗性；
- 摧毁高级异教法术书获得经验；
- 原版逐武器熟练度，以及生命、圣战、恶魔领域 build。
