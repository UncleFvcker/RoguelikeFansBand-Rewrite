# 内容包

当前内置游戏内容位于 [rfb-demo-original](rfb-demo-original/)。目录名保留历史命名，内容范围已超出早期最小演示包，不据名字判断其来源或完成度。

[pack.json](rfb-demo-original/pack.json)声明实际加载的 contentRoots；[content.lock.json](rfb-demo-original/content.lock.json)绑定版本与确定性编译 hash。Core 构建时验证并嵌入编译产物。包外的导入工具临时输出不能仅因存在于目录中就被视为正式内容。

当前数据、菜单开放范围和验收证据见[状态快照](../docs/status.md)。例如高阶法师八领域有内容与规则路径，但新游戏当前只开放死亡领域。

增加内容、核对原版、维护稳定 ID、生成 Schema 与更新内容锁，统一见[内容开发](../docs/content-development.md)。测试和 fixture 更新见[验证与契约](../docs/testing.md)。

来源及许可证按实际材料保留，原版引用不会因为放入此目录就变成项目原创授权。旧包说明保存在[文档档案](../docs/archive/2026-09-09/packs-README.before.md)。
