# Contract v253：Orc Cave 高等级审计与直接候选

`rfb-legacy-import audit-demo-monsters` 现在通过权威 RFB `master` Git 对象只读审计
21–32 级可分配怪物。每条记录输出等级、权威中英文名称、地点资格、机制 blocker、
处理状态，以及可直接写入严格选择的 ID、tags 和 omitted flags。地点规则明确排除
Camelot、其他专属地牢、`WILD_ONLY`、`WILD_OCEAN` 和 `FIXED_UNIQUE`；索引 1185
奥斯罗德始终单列为 guardian。

本批按 29–30 与 31–32 两组同步全部 45 条 direct 记录，等级分布为 15/16/5/9。
审计同时发现上一批有 10 条其他地牢或野外限定记录。它们继续保留在全局内容包中，
但移除 `orc-cave` 归属并保持原始分配范围。审计用 `imported` 与 `importedCount`
明确区分全局导入和 Orc Cave 资格：21–32 级共有 179 条已导入，其中 169 条属于
Orc Cave；审计不再留下 direct 记录，剩余状态为 29 blocked、28 excluded 和
1 guardian。VAMP、ANIM_DEAD、元素跳跃、特殊召唤、
特殊光环、变形和专属掉落表均未混入本批。

内容包升级至 1.244.0，共 595 actors、250 abilities，内容 hash 为
`038cb49ea8530a7a237a365dd24b70a62ae7474c09979800e6c8262319900bd6`。中文怪物名
严格取自 `master:src/monster_name_zh.inc`，描述取自 `master:lib/edit/r_info.txt`。
协议保持 1.169，State Hash Schema 保持 v85，save 容器保持 v1；active baseline
推进到 contract-v253，继续保留 22 个聚焦 exact fixture，不恢复旧 E2E。
