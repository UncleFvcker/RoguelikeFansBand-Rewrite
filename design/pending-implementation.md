# 待实现内容清单

状态：基于 contract-v1–v52、前端目标模式和系统路线书审计；每完成一个纵切后同步更新

本文件只记录已经在现有设计或原版对比中明确出现、但尚未实现的内容。长期设想仍保留在 [RFB 全系统梳理与重构实现路线](rfb-system-implementation-roadmap.md)，这里用于跟踪可以实际排入后续 contract 的缺口。

## 当前推进顺序

| 优先级 | 候选纵切 | 状态 | 边界 |
| --- | --- | --- | --- |
| P0 | Stage E 房间几何 | 待实现 | 房间数量、尺寸、形状、面积预算与确定性连接/回退 |
| P1 | 暂停任务管理 | 待实现 | 地表直接放弃、重接次数限制和重新生成策略 |

## contract-v32 明确遗留

- 解除失败触发陷阱、重复解除命令和经验奖励；
- 箱子陷阱、随机陷阱类型、状态/传送/落层等复杂效果；
- 被动搜索、失明/无光/混乱/幻觉修正，以及怪物触发或规避陷阱；
- 一次性/耗尽陷阱、陷阱生成密度和多深度内容表。

来源：[contract-v32](contract-v32-hidden-traps-disarm.md)。

## contract-v33 明确遗留

- 镐、铲、重武器等装备提供的挖掘能力与物品描述；
- 自动重复挖掘、疲劳、声音、德行和挖掘秘密门时的偶发搜索；
- 树木、矿脉、玻璃、永久岩石等不同破坏规则与产物；
- 原版“怪物挡路时转为攻击”的兼容语义；当前核心与权威查询统一拒绝被占据目标。

来源：[contract-v33](contract-v33-diggable-terrain.md)。

## contract-v34 明确遗留

- 分支楼梯、同层多个连接点、连接 ID 与到达点分别建模；
- 随机楼梯位置、回忆/传送等非楼梯跨层入口；
- 深度相关 encounter/loot/theme 表已由 contract-v48 完成；尚缺分支连接。守护者、最终层和禁止下行规则已由 contract-v46 完成；
- 旧 v33 已访问深度 1 不补下楼梯，因此不能从旧存档进入新深度 2；需要正式存档迁移策略。

来源：[contract-v34](contract-v34-multi-depth-floors.md)。

## contract-v35 明确遗留

- 多座地牢各自独立的活跃探索实例与同时存在规则；
- 地牢中途传送回城、回忆、死亡或任务退出时统一结束探索；
- 明确的 `DungeonInstanceId`，以及分支入口对应不同实例；稳定 `dungeonId` 与持久守护者状态已由 contract-v46 完成；
- 可配置的永久地牢或重置策略；当前所有程序化地牢从入口返回地表都会清除。

来源：[contract-v35](contract-v35-dungeon-expedition-lifecycle.md)。

## contract-v36 明确遗留

- 当前退出一次性任务层即关闭入口，尚未区分完成、失败和放弃；
- 护送和到达位置目标；有序多阶段任务已由 contract-v45 完成，击杀与收集目标已由 contract-v37/v39 建立；
- 任务奖励、任务日志、重新接取和可重复任务；
- 固定手工地图、任务专属生成器及禁止离开的任务规则。

来源：[contract-v36](contract-v36-one-shot-task-floor.md)。

## contract-v37 明确遗留

- 计数收集、护送和到达位置目标；有序多阶段任务已由 contract-v45 完成，单实例击杀已由 contract-v39 完成；
- 主动放弃与失败的区分、禁止提前退出已由 contract-v40 完成；尚缺超时失败；
- 任务奖励、任务日志、重新接取和可重复任务；
- 目标物被丢弃、销毁、投掷或带出后再处理的完整规则。

来源：[contract-v37](contract-v37-task-objective-resolution.md)。

## contract-v38 明确遗留

- 奖励选择、随机奖励、领取确认和容量不足处理；
- 多任务排序、任务详情与历史记录；目标进度数字已由 contract-v39 完成；
- 可重复任务、重新接取、奖励已领取状态；
- 奖励和任务状态改为独立领域状态，而不是完全由 terrain/物品派生。

来源：[contract-v38](contract-v38-task-reward-journal.md)。

## contract-v39 明确遗留

- 按怪物种类累计多次击杀与持久计数已由 contract-v41 完成；尚缺跨楼层共享同一任务进度；
- 清空楼层、unique、随机目标、召唤物过滤和更通用的死亡订阅边界；
- 主动放弃与退出限制已由 contract-v40 完成，可重接暂离由 contract-v42 完成；尚缺超时失败和失败后的重新接取；
- 一个阶段内的多目标、分支阶段，以及独立 quest 模块；有序多阶段任务和持久任务状态已由 contract-v45/v44 完成。

来源：[contract-v39](contract-v39-kill-objective-progress.md)。

## contract-v40 明确遗留

- 可重接任务和保留任务层已由 contract-v42 完成；尚缺失败后的重新接取流程；
- 离开前确认对话框，以及失败/放弃后的惩罚或脚本回调；
- 死亡、回忆、传送和其他非楼梯离开方式统一进入任务结算；
- 超时失败，以及禁止离开但仍允许主动放弃之外的更多退出政策。

来源：[contract-v40](contract-v40-task-abandon-exit-policy.md)。

## contract-v41 明确遗留

- 多个入口楼层共享任务 ID 和计数状态已由 contract-v43 完成；尚缺任务内部的上下层连接；
- 可重接任务离开、重新进入后继续累计已由 contract-v42 完成；尚缺暂停时在地表主动决定最终失败；
- 召唤物、复制体、非玩家击杀和环境击杀的可配置计数规则；
- unique、随机任务、清空楼层和阶段内多目标；有序多阶段任务已由 contract-v45 完成。

来源：[contract-v41](contract-v41-counted-kill-progress.md)。

## contract-v42 明确遗留

- 独立于 floor ID 的任务 ID 和多个入口楼层共享任务已由 contract-v43 完成；
- 暂停状态下从地表直接最终放弃、重接次数限制和超时；
- 重入时选择保留完整楼层、只保留进度或重新生成普通目标的内容策略；
- 任务接取确认、失败惩罚和脚本回调。

来源：[contract-v42](contract-v42-retakeable-task.md)。

## contract-v43 明确遗留

- 同一任务内的直接上下层连接；并列入口之间的有序阶段已由 contract-v45 完成；
- 一个阶段内声明多个同时目标；跨成员楼层的收集、进入和击杀组合已由 contract-v45 完成；
- 独立任务名称、描述、接取来源和任务详情领域实体；
- 多任务并行追踪、排序、筛选和当前追踪目标。

来源：[contract-v43](contract-v43-shared-task-id.md)。

## contract-v44 明确遗留

- 分支、可选、并行阶段和一个阶段内的多个同时目标；有序单目标阶段已由 contract-v45 完成；
- 将任务状态机从游戏聚合继续拆分为独立 quest 模块；
- 任务接取来源、详情、脚本回调、超时与失败惩罚；
- 多任务追踪选择，以及非玩家/环境击杀的可配置计数来源。

来源：[contract-v44](contract-v44-task-state-machine.md)。

## contract-v45 明确遗留

- 分支、可选和并行阶段，以及一个阶段内的多个同时目标；
- 阶段级奖励、失败政策、脚本回调与阶段描述文本；
- 暂停状态下从地表主动放弃、重接次数限制和成员楼层重新生成策略；
- 独立 quest 模块、任务接取来源与多任务追踪选择；
- 更通用的到达位置目标，以及环境/非玩家击杀的可配置计数来源。

来源：[contract-v45](contract-v45-ordered-task-stages.md)。

## contract-v46 明确遗留

- 分支、跳层、shaft、随机楼梯和独立到达点；当前地牢链仍是严格线性；
- vault 内的深度 encounter、主题 terrain/loot 和固定群体已由 contract-v47 建立；楼层级表、多个 vault 加权选择和第一类巢穴已由 contract-v48 建立；十层规模、actor/loot 预算和深度区域主题已由 contract-v49 建立；旋转/镜像、自由落位、多 Vault 空间预算和失败回退已由 contract-v50 建立；动态 friends/escort formation 与群体预算已由 contract-v51 建立。尚缺多入口与大模板成功落位后的连通性证明；
- 入口守护者、守护者 unique 世界生态，以及神器、声望和属性奖励；
- 多座地牢、进入条件、显式 `DungeonInstanceId`、胜利/退休和角色分数；
- 回忆、传送、死亡等非楼梯方式结束探索时的统一生命周期。

来源：[contract-v46](contract-v46-final-floor-guardian.md)。

## contract-v47 明确遗留

- 按深度和地牢主题加权选择多个 vault、无候选回退已由 contract-v48 建立；旋转、镜像、自由 wall 区落位、多 Vault 同层和生成失败回退已由 contract-v50 建立，尚缺多入口和跨走廊拼接；
- 普通房间可引用的独立 encounter/loot/theme 表已由 contract-v48 建立，actor/loot 总预算与深度区域主题已由 contract-v49 建立，第一版 Vault 数量/面积预算已由 contract-v50 建立，额外 trap/door/rubble 表与 feature 预算已由 contract-v52 建立，房间数量/形状/面积预算与连通 cavern 基础地貌已由 contract-v53 建立；尚缺 lake/river/streamer/destroyed/maze、群落和同层多区域主题；
- 第一类同类巢穴已由 contract-v48 建立，动态 friends/escort、`cluster/ring` formation 和群体预算已由 contract-v51 建立；尚缺 pit、任意形状 formation、召唤、繁殖、pack AI、种群上限和 unique 过滤；
- vault 越级强敌/掉落、专属陷阱、神器、来源标签和探索奖励；
- 十层规模压力场景已由 contract-v49 建立，多 Vault 楼层已由 contract-v50 建立；尚缺更大模板成功落位后的连通性证明和多入口。

来源：[contract-v47](contract-v47-themed-vault.md)。

## contract-v48 明确遗留

- 十层地牢、actor/loot 生成预算和深度区域主题已由 contract-v49 建立，多个 Vault 同层和第一版面积预算已由 contract-v50 建立，额外陷阱/门/障碍空间预算已由 contract-v52 建立，房间几何预算与 cavern 基底已由 contract-v53 建立；尚缺其他原版式地貌阶段与机器性能计时基线；
- Vault 旋转、镜像、自由 wall 区落位和失败重试已由 contract-v50 建立；尚缺多入口与大模板成功落位后的连通性证明；
- 动态 friends/escort、`cluster/ring` formation 与领袖/随从预算已由 contract-v51 建立；尚缺巢穴专属表、任意形状、主题掉落、pit 和 pack AI；
- unique/守护者过滤、召唤物与繁殖种群上限、越级强敌/掉落和神器来源标签；
- 分支、shaft、随机楼梯、同层多个连接点与显式到达点。

来源：[contract-v48](contract-v48-floor-generation-tables.md)。

## contract-v49–v53 明确遗留

- Vault 旋转、镜像、自由 wall 区落位、多 Vault 预算竞争、重叠拒绝和稳定失败回退已由 contract-v50 建立；尚缺多入口、大模板成功落位后的连通性证明和跨走廊拼接；
- 额外陷阱、门与可挖掘特殊地形表、room/corridor 放置、空间预算和失败回退已由 contract-v52 建立；房间数量/尺寸/rectangle-cross 形状/面积预算、连通 cavern 基地貌和跨房间内容分布已由 contract-v53 建立；尚缺 lake/river/streamer/destroyed/maze、feature 分类型配额、相邻限制与同层多区域主题拼接；
- friends/escort、`cluster/ring` formation、群体数量/随从预算、空间缩减和原子回退已由 contract-v51 建立；尚缺 pit、任意形状 formation、pack AI、召唤、繁殖、种群上限和 unique 过滤；
- 分支、shaft、随机楼梯、同层多个连接点与独立到达点；
- 跨机器性能计时基线；当前十层 fixture 只锁定规模、状态和确定性。

来源：[contract-v49](contract-v49-budgeted-pressure-dungeon.md)、[contract-v50](contract-v50-spatial-vault-placement.md)、[contract-v51](contract-v51-dynamic-encounter-groups.md)、[contract-v52](contract-v52-terrain-feature-budgets.md)、[contract-v53](contract-v53-staged-cavern-layout.md)。

## contract-v25–v29 明确遗留

### 怪物携带物与掉落

- 偷窃、缴械、怪物主动拾物和怪物使用物品；
- 多次掉落、区域主题掉落、unique 过滤和特殊怪物掉落规则；楼层 loot 表引用已由 contract-v48 建立，vault 专属 loot 已由 contract-v47 建立；
- 统一 `DeathOutcome` 订阅边界，以及经验、任务、统计等死亡消费者。

来源：[contract-v25](contract-v25-monster-carried-items.md)、[contract-v24](contract-v24-deterministic-loot-generation.md)。

### 楼层与生成

- 多深度连接、任务层、临时/持久层策略和旧层淘汰；
- 动态朋友/护卫群体、`cluster/ring` formation 与群体预算已由 contract-v51 完成，额外陷阱/门/可挖掘障碍表与空间预算已由 contract-v52 完成，房间几何预算、连通 cavern 基底与跨房间内容分布已由 contract-v53 完成；尚缺同层多区域主题和 lake/river/streamer/destroyed/maze 等生成阶段。最终层守护者已由 contract-v46 完成，第一类固定主题 vault/group 已由 contract-v47 完成，多 Vault 加权选择与第一类巢穴已由 contract-v48 完成，actor/loot 总预算和十层压力链已由 contract-v49 完成，Vault 变换、自由落位、多模板面积预算与失败回退已由 contract-v50 完成；
- 陷阱、秘密门和其他可变地形进入生成管线。

来源：[contract-v26](contract-v26-floor-lifecycle.md)、[contract-v27](contract-v27-procedural-room-content.md)。

### 门与地形交互

- 原版 easy-open/自动选方向；权威可查询交互列表已由 [contract-v30](contract-v30-authoritative-terrain-interactions.md) 完成；
- 卡死门、玻璃门、更复杂的秘密门变体和门上的声音/经验反馈；
- 开锁受失明、无光、混乱、幻觉影响；
- 撞门成功后自动进入门洞、普通开启/破损随机分支；
- 撞门失败后的失衡/麻痹；
- 怪物挡门时是否显式转为近战，以及怪物自身开门/破门 AI。

来源：[contract-v28](contract-v28-door-terrain-state.md)、[contract-v29](contract-v29-locked-door-checks.md)。

### 搜索与地形知识

- 基础秘密门、主动搜索和知识安全投影已由 [contract-v31](contract-v31-secret-door-search.md) 完成；
- 尚未实现被动搜索、搜索模式/命令重复、玩家自身格搜索和固定 3×3 RNG 扫描；
- 失明、无光、混乱、幻觉对搜索能力的修正；
- 隐藏陷阱和箱子陷阱发现。

## 更早纵切遗留

### 战斗、状态与效果

- 玩家 on-hit effect、暴击、品牌、克制、吸血等武器效果；
- 怪物 blow 的多 effect 列表、位移与中断；
- 失明、混乱、麻痹，以及这些状态对行动和检定的统一修正；
- 自然恢复、饥饿、休息、环境伤害和世界级 tick 回调；
- 抗性与感知进入更完整的多来源派生属性。

来源：[contract-v9](contract-v9-status-resistance-effects.md) 至 [contract-v13](contract-v13-monster-melee-routines.md)。

### 射击、投掷与目标选择

- 特殊返回弹药/武器、职业折损修正和职业射击修正；
- 药水投掷破裂与落点 effect；
- 投掷目标模式、鼠标点选、路径/范围预览和投射物动画；
- 自身、范围、锥形等 `TargetSpec` 模式。

来源：[contract-v14](contract-v14-projectile-foundation.md) 至 [contract-v18](contract-v18-thrown-attacks.md)、[前端目标模式 v1](frontend-targeting-v1.md)。

### 背包、装备、鉴定与物品

- 身体槽位扩展、箭袋、容器、槽位容量和负重分级惩罚；
- 鉴定卷轴、鉴定技能、诅咒知识、伪鉴定来源和逐项属性发现；
- unique、诅咒、固定神器、随机能力、随机神器和重铸；
- 消耗品的目标、状态 effect 与多 effect 组合。

来源：[contract-v19](contract-v19-inventory-capacity.md) 至 [contract-v24](contract-v24-deterministic-loot-generation.md)。

### 怪物 AI 与知识界面

- `AiIntent`、保持距离、逃跑、守卫、射击、能力选择、气味/flow 和特殊感知；
- 怪物开门、破门、拾物、偷窃和缴械决策；
- 怪物、物品、神器、ego、地形、地牢、宠物和统计知识菜单。

来源：[RFB 全系统梳理与重构实现路线](rfb-system-implementation-roadmap.md)。

## 维护规则

- 新纵切开始时，把目标列入“当前推进顺序”并标记进行中；完成后从待实现项中移除或标记由哪个 contract 完成。
- 每次原版对比发现主动差异时，必须判断它是永久设计差异还是未来缺口；未来缺口写入本文件。
- 不把“可能有用”的新功能直接加入清单；必须能追溯到既有规划、已完成 contract 的延后说明或原版对比。
