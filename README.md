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
- [本地化与中文文本重构计划](design/localization-rewrite-plan.md)

## 原项目

旧版 RFB 源码和当前可玩版本继续保留在：

[UncleFvcker/RoguelikeFansBand-zh-CN](https://github.com/UncleFvcker/RoguelikeFansBand-zh-CN)

旧项目在重构期间作为规则行为、数据内容和旧存档兼容的参考实现。

## 当前阶段

当前处于架构和协议准备阶段。下一步是建立 Cargo workspace、`rfb-core`、`rfb-protocol`、Rust/WASM 桥接以及 TypeScript/PixiJS 前端骨架。

