# RFB Pixel 28 美术规范

本目录保存 RFB 正式像素 tileset 的唯一可编辑源文件。运行时 PNG 在后续步骤中从该源文件导出，不维护 56×56 或其他分辨率的第二套素材。

通用的选材、概念参考、Aseprite 导出、manifest 接入和验收流程见 [`../../../design/tileset-production-guide.md`](../../../design/tileset-production-guide.md)。

## 技术规格

- 主图集源文件：`rfb-pixel-28.aseprite`
- 玩家源文件：`players/warrior.aseprite`；运行时导出为独立的 `players/warrior.png`
- 画布：224×448 像素
- 网格：8 列 × 16 行，每格 28×28 像素，原点为 `(0, 0)`
- 色彩模式：RGBA；源文件内嵌基础色和项目扩展色
- 帧数：1；首版不制作动画
- 图层：单一 `Artwork` 图层；不把运行时光照或可见性效果画入源文件
- 1× 显示为 28×28；2× zoom 仅使用最近邻放大为 56×56
- 导出必须保持 224×448，不裁边、不旋转、不留格间距或外边距

## 色板策略

源文件内嵌 36 个基础色，但不限制素材只能使用这些色值；允许为材质区分、轮廓清晰度和关键对象辨识增加项目色。

基础色值：

```text
#25131a #3d253b #523b40 #1f3736 #2a5a39 #427f3b
#80a53f #bbc44e #96c641 #ccf61f #8a961f #5c6b53
#895a45 #d1851e #ffd569 #bf704d #e1a171 #e6deca
#9b4c51 #802954 #d01946 #e84444 #40369f #7144ff
#af69bf #eaa5ff #5880cc #62abd4 #9bf0fd #cae6f5
#ffffff #a7acba #606060 #56587b #9a8571 #dfbbb3
```

首批项目扩展色：

```text
#171820 #242936 #343b48 #4b5665 #687789 #91a0ad
#24588a #2f7fb4 #45b7d1 #a6f2f5 #d49a6a #7b4935
#8f4e1b #c87820 #f2b632 #ffe27a #fff2b3
#1b2d2e #2c3938 #3e4946 #505a54 #636b63 #787e73
#969a8c #b4b6a5
```

荒野地形扩展色：

```text
#392d2b #4a3a32 #604b3d #786149 #967a5c #b19470
#182622 #1f3432 #28372a #35452d #4d5a31 #6e7137
#91aebc #b6cbd3 #d7e3e7 #eef4f3 #778f9a #596653
#35677f #5795ad #6eafc3 #8fc8d8 #b8e0e8 #d8f0f2
#252a31 #343b43 #48515a #626b72 #83898a #a0a19b
#235875 #347e9d #4fa8c3 #74c8dc #a3e2ea #d1f2f2
#261b1b #3b2421 #5b2c22 #8c3520 #c84b1e #f07822
#ffb431 #ffe06a
```

新增颜色必须有明确用途，不添加肉眼难以区分的近似色；新增色值同时写入本节和 Aseprite 源文件色板。单个 tile 通常使用 6–12 色，每种材质使用 3–5 级明暗，确有辨识需要时才能突破这个范围。

首批四格沿用概念稿的配色方向：石材采用冷灰绿和深青灰裂缝，玩家采用高饱和蓝青盔甲并以少量暖色皮革点缀，金币使用深棕轮廓、橙金中间色和淡黄高光。地板与墙以 28×28 缩小稿作为构图底样，再手工减色、整理裂缝和砖缝；不直接保留缩放产生的近似色。

## 视觉规则

- 所有素材统一使用左上方光源。
- 地形保持低对比；角色、物品和可交互物使用更清晰的轮廓与更强对比。
- actor 的主色不得与其常见地形背景相同；在深色和浅色地形上都要检查轮廓。
- 玩家倾向蓝青与亮中性色，敌对单位倾向红与暖紫，友方倾向绿与浅中性色，可交互物倾向金黄；魔法交互物可使用紫色。这些是识别倾向，不要求整块填成阵营色。
- 地形 tile 可以铺满 28×28；actor 和物品通常保留约 2 像素透明边缘。
- 绘制时不使用抗锯齿，不缩放笔刷或素材，不产生半像素。
- 普通像素使用完全不透明或完全透明；不手绘半透明阴影、光晕或照明。
- 不把黑暗、FOV、记忆区色调或火把颜色画入素材，这些由运行时图层处理。
- 不把物品或怪物烘焙进地形；terrain、object、actor、visibility、lighting 保持独立。

## 固定坐标

以下名称是美术工作标签，不是玩家界面的显示名称。首版发布后不移动已有坐标；后续内容从新行继续追加。

| y\\x | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| 0 | dungeon_floor | dungeon_wall | permanent_wall | magma_vein | quartz_vein | surface_grass | surface_path | surface_rock |
| 1 | outpost_wall | outpost_gate | door_closed | door_open | door_locked | door_broken | stairs_up | stairs_down |
| 2 | player_warrior（外部 PNG，格位保留） | newt | giant_white_mouse | small_kobold | kobold | warg | warrens_keeper | wild_cat |
| 3 | trap | gold | ration | torch | broad_sword | arrow | healing_potion | fur_cloak |
| 4 | surface_tree | surface_water_shallow | surface_water_deep | surface_woodland | temple_wall | door_secret | warren_snare | thieves_hideout_entry |
| 5 | general_store_entrance | armoury_entrance | weaponsmith_entrance | temple_entrance | alchemist_entrance | arcane_shop_entrance | black_market_entrance | home_entrance |
| 6 | raven | duck | fruit_bat | giant_white_centipede | jackal | rock_lizard | blue_yeek | giant_green_frog |
| 7 | brass_lantern | flask_of_oil | broken_weapon | club | dagger | generic_scroll | cloth | shovel |
| 8 | freesia | metallic_green_centipede | salamander | metallic_blue_centipede | metallic_red_centipede | cave_lizard | giant_white_rat | large_kobold |
| 9 | giant_brown_bat | rat_thing | night_lizard | brown_yeek | giant_salamander | giant_grey_rat | skaven | skaven_shaman |
| 10 | mace | light_sword | whip | small_leather_shield | generic_headgear | generic_gloves | generic_boots | leather_armour |
| 11 | purple_potion | blue_potion | green_potion | speckled_potion | cloudy_potion | fungal_food | corpse_remains | skeleton_remains |
| 12 | surface_waste | surface_swamp | surface_snow | surface_pack_ice | surface_mountain | surface_glacier | surface_lava_shallow | surface_lava_deep |
| 13 | filthy_street_urchin | agent_of_black_market | novice_rogue | scruffy_looking_hobbit | nibelung | bandit | tax_collector | count_entrance |
| 14 | floating_eye | grip_farmer_maggots_dog | wolf_farmer_maggots_dog | fang_farmer_maggots_dog | blubbering_icky_thing | cave_spider | clear_icky_thing | giant_black_ant |
| 15 | goomba | large_yellow_snake | shrieker_mushroom_patch | slimy_worm_mass | white_harpy | yellow_jelly | yellow_mushroom_patch | giant_white_ant |

当前 224×448 atlas 的 y0–y15 已全部分配。共鸣区域属于测试 demo，不纳入正式 tileset。`arcane_shop_entrance` 由书店和魔法店共用，盗贼藏身处各状态暂时共用一个入口图块；`broken_weapon` 由两种破损武器共用，`generic_scroll` 由 Warrens 掉落卷轴共用，`cloth` 由 Warrens 的布质衣甲共用。`light_sword`、头具、手套、靴子和皮甲分别由同类 Warrens 装备共用；主匕首和短刀复用 `dagger`，镐复用 `shovel`。第 11 行的五种药剂按外观色系复用，史莱姆霉和四种蘑菇共用 `fungal_food`；轻伤治疗药剂复用第 3 行的 `healing_potion`。`crow` 和 `crow_of_durthang` 复用 `raven`；四种活动钱币复用第 3 行的金币堆。

## 绘制与验收顺序

1. 先绘制 `(0,0)` 地板、`(1,0)` 墙、`(0,2)` 玩家和 `(1,3)` 金币。
2. 导出当前完整尺寸 PNG，用这四格验证地形、actor、object、透明边缘、FOV、光照和 glyph fallback。
3. 最小切片通过后，再按第 0、1、2、3 行的顺序补齐。
4. 每次交付同时检查 1× 和 2×；出现模糊、接缝或缩放插值即视为失败。

## 玩家素材

- 玩家形象不再打包进主 atlas；`(0,2)` 保持透明保留位，避免移动后续怪物坐标。
- 每种玩家形象使用独立的 28×28 RGBA PNG 和同名 Aseprite 源文件，统一放在 `players/` 下。
- manifest mapping 使用 `image` 安全相对路径引用玩家 PNG；透明像素必须显示下方地形，不声明 mapping background。
- 新形象使用稳定的语义化文件名，不通过扩展主 atlas 添加玩家变体。
