# 地形默认配色

三个内置图块方案都为当前局部地形及 18 个世界地图视觉 ID 提供明确的前景／背景色。ASCII 模式直接使用，图片模式保留原图资源，并用于未提供图片的地形及图片加载失败时的字符显示。粉色仍用于真正缺失的映射。

来源为 `D:/codex/Frogcomposband/master` 的 `master` 提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`，通过 Git 对象读取 `lib/edit/f_info.txt` 的 `G:` 配色、`src/variable.c` 的颜色表以及 `src/wild.c` 的 `init_terrain_table` 世界地形对应关系。此处是颜色改编，不导入原版源码或改动地形规则。

沿用原版的色系：水域蓝／青、植被绿、泥土棕、永久墙浅棕、花岗岩与石英灰白、门棕、熔岩红／橙、冰雪白／浅蓝、地牢入口紫。针对深色背景和缩小后的字符，提高纯蓝、暗棕等颜色的亮度；草地与森林、道路与荒地、山地与冰川分别采用不同明度和底色。原版沼泽与浅水同为浅蓝，改编中把沼泽偏向灰绿青。背景是新版补充的低饱和色块，不声称原版已有这些背景。

城镇使用暖白标记与较亮棕底，入口保留紫色。局部任务入口沿已公开的字符／材料显示：入口箭头紫色，关闭建筑沿墙色，地面沿地板或植被色；不凭未发现的任务或隐藏矿物状态额外着色。隐藏地形仍由核心的已知地形投影遮蔽。

默认值只存于 `web/public/tilesets/*/tileset.json`。不修改角色存档、全局偏好文件或基本调色板；用户已保存的字形／配色覆盖继续优先。

验证：在 `web` 执行 `node --test src/tileset-manifest.test.ts src/visual-preferences.test.ts src/render-world.test.ts`，覆盖现有地形完整映射、颜色区分、隐藏矿物外观及偏好覆盖。普通 standalone 构建后执行 `node e2e/global-preferences-standalone.e2e.mjs --terrain-colors`，正常新建人类战士，查看局部地图、进入世界地图并切换图片方案；报告与截图在 `test-results/terrain-colors/`。脚本复用偏好验收的启动／退出和原偏好恢复流程，运行时不能同时打开其他游戏进程。
