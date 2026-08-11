# Contract v252：Orc Cave 直接候选与冷血红外规则

原版 `RF2_COLD_BLOOD` 现映射为 actor tag `cold-blooded`。红外视觉不会揭示该类
怪物；若怪物同时具有火焰接触光环，则沿用原版例外，仍可由红外视觉发现。普通视觉、
心灵感应和其他感知管线不受影响。

Orc Cave 第二批从权威 RFB `master` 直接导入 21–28 级的 134 条可表达记录，等级分布
为 15/15/12/16/23/16/18/19。同步选择由 `orc-cave` tag 标识，并生成所需的 48 个
参数化怪物能力；中文怪物名严格来自原版运行时名称表。29–32 级候选、固定荒野 Unique
Old Man Willow 与最终守卫奥斯罗德不在本批范围。

协议保持 1.169，State Hash Schema 保持 v85，save 容器保持 v1。内容包升级至
1.243.0，共 551 actors、233 abilities，内容 hash 为
`8966d8580dac15e533a4caa84b8828effa2a843a06acc1da0527fc2fe405b27d`。active baseline
推进到 contract-v252，保留 22 个聚焦 exact fixture，不恢复旧 E2E。
