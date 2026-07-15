# RoguelikeFansBand Rewrite

RoguelikeFansBand 的新一代重构工程。

本仓库不直接复制旧 C 工程，而是以稳定协议和行为测试为边界，逐步重新实现游戏核心与前端。

## 技术方向

- Rust：游戏规则、数据模型、随机数、存档、AI 与原生核心
- TypeScript + Vite：Web 前端和开发工具
- PixiJS：地图、tileset、光照与动画渲染
- WebAssembly + Web Worker：浏览器中的 Rust 核心
- Tauri：Windows、Linux 和 macOS 桌面封装
- Fluent：英文/简体中文本地化

## 设计文档

- [HTML/Rust 重构计划](design/html-rewrite-plan.md)
- [旧版行为基准与差分测试](design/legacy-behavior-baseline.md)
- [核心协议 v1](design/protocol-v1.md)
- [确定性模拟、随机数与回放](design/deterministic-simulation.md)
- [内容数据格式 v1](design/content-format-v1.md)
- [新存档格式 v1](design/save-format-v1.md)
- [授权、版权与素材迁移审计](design/licensing-and-assets.md)
- [本地化与中文文本重构计划](design/localization-rewrite-plan.md)

## 原项目

旧版 RFB 源码和当前可玩版本继续保留在：

[UncleFvcker/RoguelikeFansBand-zh-CN](https://github.com/UncleFvcker/RoguelikeFansBand-zh-CN)

旧项目在重构期间作为规则行为、数据内容和旧存档兼容的参考实现。

## 当前阶段

P0 架构规范已经建立。下一步先针对旧版 `v1.3.0.7` 实现行为基准 manifest、命令回放和首批 golden fixtures；达到阶段 0 门槛后，再开始批量迁移 Rust 规则模块。
