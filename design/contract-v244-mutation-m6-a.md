# Contract v244: mutation M6-A

状态：已实现。协议 `1.165`，save 容器 `v1`，State Hash Schema `v82`，内容包
`1.236.0`，内容 hash
`741e3324a94567a6c0032baff773182ca2823be675296d9fc3784b82b9040071`。

M6-A 激活十项原版周期变异：Berserk Rage、Cowardice、Alcohol、Hallucination、
Produce Mana、Speed Flux、Invulnerability、SP→HP、HP→SP 与 Hypochondria。
处理只发生在本地地图，按 `sourceIndex` 排序，并保持前置抗性、触发判定和后续
RNG 的原版调用顺序。

Produce Mana 使用单槽 `pendingMutationDirection` 暂停周期序列与当前 tick；Web
复用八方向瞄准器，解析后从 Produce Mana 后一项继续。Speed Flux 的 `minorSlow`
和待选方向均保存并参与状态哈希。Hypochondria 的 `unwell` 复用状态系统，在有效
阶段修改敏捷、体质，并以现有冰冷 beam 事务处理喷嚏。

验收由 Core 固定种子测试覆盖状态、资源、方向恢复、存档和状态哈希；Web 测试
覆盖强制方向选择、命令门控和目标投影。21 个 active contract fixture 因公共协议
与哈希结构统一刷新，保持零 waiver。
