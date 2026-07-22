# Contract v32：隐藏陷阱、触发与解除

状态：协议 1.32 / contract-v32 active baseline

## 已完成边界

- `TerrainDefinition.trap` 强类型声明固定伤害、伤害类型、解除结果 terrain 和解除难度；程序化首层在入口南侧稳定放置隐藏回声绊索。
- 未发现时陷阱投影为地板且不输出解除交互；主动搜索成功或玩家踩入后复用 contract-v31 知识投影公开。
- 踩入后复用 `DamagePacket`、抗性和结构化 damage/death outcome，固定造成 2 点物理伤害；陷阱保持武装直到解除。
- `DisarmTrap` 使用独立 `disarmSkill` 和结构化检定；失败保持，成功转换为地板。前端大写 `D` 解除，小写 `d` 保留东移。

协议 1.32，内容包 1.26.0，content hash `224e4cc12f1f1a99e245b5e1a96e7c9371a6873460b6197c0f18007542c1a079`，terrain 10。save v1 与 state-hash Schema v15 不变。active baseline 迁移 68 个并新增两个场景，共 70 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `cmd1.c::search()` 也在搜索成功后公开隐藏陷阱；`cmd2.c::do_cmd_disarm_aux()` 使用独立解除技能对抗陷阱 power，成功移除、部分失败保持；`cmd1.c::hit_trap()` 在玩家进入真实陷阱格后触发效果和伤害。

主动差异：重构明确分离陷阱真值、玩家知识与权威交互查询，并使用稳定内容 ID、强类型伤害和跨平台回放/hash。初版解除失败不会像原版严重失败那样直接触发陷阱，也不会自动向陷阱格移动。

暂未实现：失明/无光/混乱/幻觉修正、经验与重复命令、箱子陷阱、随机陷阱表、落层/传送/状态等复杂效果、怪物触发或规避，以及被动搜索。
