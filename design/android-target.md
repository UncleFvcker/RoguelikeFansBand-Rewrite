# Tauri Android 原生目标

状态：ARM64 Debug APK 构建链已建立；真机交互验证尚未完成

## 1. 已建立的边界

Android 与 Windows 使用同一个 `rfb-tauri` crate、`AppState`、Tauri Commands、Rust 游戏核心和 TypeScript/PixiJS 前端。Android 工程只负责 Activity、WebView、Gradle 打包和 JNI 动态库装载，不复制或重新实现游戏规则。

当前配置：

- application ID：`io.github.unclefvcker.rfb_rewrite`；
- minSdk：24；
- compileSdk/targetSdk：36；
- 首个持续验证 ABI：ARM64 (`aarch64-linux-android`)；
- Java：21；
- Android Gradle Plugin：8.11.0；
- Gradle：8.14.3；
- NDK：29.0.13846066；
- 正式前端资源嵌入 APK，不依赖浏览器服务器。

生成工程位于 `web/src-tauri/gen/android`。Gradle 缓存、生成的 Kotlin/JNI 文件、APK 和本地签名配置均由内部 `.gitignore` 排除。

## 2. 本地依赖

本地需要 Android SDK command-line tools，并安装：

```text
platform-tools
platforms;android-36
build-tools;35.0.0
ndk;29.0.13846066
```

Rust 需要：

```powershell
rustup target add aarch64-linux-android
```

仓库默认也可使用不提交的 `.local/android-sdk`。本机混合代理示例：

```powershell
.\scripts\build-android.ps1 -Proxy http://127.0.0.1:7897
```

若 SDK 位于其他位置：

```powershell
.\scripts\build-android.ps1 -SdkPath D:\Android\Sdk
```

成功后 APK 位于：

```text
web/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

Debug APK 未优化且包含调试符号，不用于发布。正式 AAB/APK 需要先建立密钥、签名保管和 release shrinking 验证。

## 3. 当前验证结果

- Tauri Android 工程成功初始化；
- `rfb-tauri` 和所有核心 crate 成功为 ARM64 Android 编译；
- `librfb_tauri_lib.so` 成功装入 APK 的 `lib/arm64-v8a/`；
- APK manifest 为 minSdk 24、targetSdk 36；
- 前端 `build:ui`、Rust 原生命令和 Android Gradle 打包在同一命令内完成；
- APK 检查未发现旧 RFB/FrogComposband 文件或资源。

本阶段没有模拟器或真机，因此尚未声称触控、生命周期、文件选择器和低内存恢复已经通过。

## 4. CI

GitHub Actions 的 Android job 安装固定 NDK 和 ARM64 Rust target，构建 Debug APK 并上传为工作流产物。CI 只验证可重复编译与打包，不发布或签名应用。

## 5. 下一步

1. 在 Android 真机安装 APK，验证启动、PixiJS/WebView2 替代实现、存档与回放；
2. 设计触屏操作层，保持小键盘、Vi、WASD 键位预设不被挤占；
3. 针对窄屏重新组织地图、状态和消息面板，而不是简单缩小桌面布局；
4. 验证切后台/恢复、屏幕旋转、低内存重建和文件选择器权限；
5. 建立正式签名、AAB、版本码和发布渠道后再产生 Android release。
