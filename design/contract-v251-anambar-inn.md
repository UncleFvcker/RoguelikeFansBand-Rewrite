# Contract v251：阿南巴旅店住宿

阿南巴旅店提供原版“住宿一晚”服务。玩家必须站在旅店入口，支付 25 金币，且当前
不能中毒或流血。成功后城镇时间直接推进到下一个 50,000 tick 半日边界；生命、职业
资源和背包设备充能恢复至上限，临时状态、轻微减速、延迟现实改变与生效中的召回被
清除。该时间跳跃不循环执行怪物、周期变异或环境 RNG。

`StayAtInn { facilityId }` 是唯一新增命令。失败返回 `inn.stay-unavailable`，不扣费、
不推进 `worldTick`、不消费 RNG。成功返回 `inn.stay`。前端只在阿南巴旅店交易面板
显示住宿按钮，继续复用现有商店面板和事件消息。

协议为 1.169，State Hash Schema 保持 v85，save 容器保持 v1，内容包保持 1.242.0。
active baseline 为 contract-v251，共 22 个 exact fixture；新增 fixture 只固定跨协议的
入口不可达拒绝，成功恢复和拒绝原子性由聚焦 Core 测试覆盖。按项目约定不运行旧 E2E。
