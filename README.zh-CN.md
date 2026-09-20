<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="Quest Manager 图标">
</p>

<h1 align="center">Quest Manager</h1>

<p align="center">
  通过本地 ADB 连接，管理 Meta Quest 上的应用与文件。
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong>
</p>

<p align="center">
  <a href="docs/README.md">文档导航</a> ·
  <a href="AGENTS.md">Agent 指南</a> ·
  <a href="SECURITY.md">安全说明</a> ·
  <a href="PRIVACY.md">隐私说明</a>
</p>

用于管理 Meta Quest 的本地 Windows 桌面应用，使用 **Tauri 2 + React + TypeScript + Rust + 官方 ADB**。界面为英文，暂不引入 i18n。当前优先支持 Windows x64。

开发与使用体验以 **Quest 3** 为主，不限制头显机型；其他机型可以使用其 ADB 与 Android 权限所支持的操作。Overview 会根据识别到的型号显示 Quest 3、Quest 3S 或 Quest 2 的对应插图，未知型号使用通用插图。

<p align="center">
  <img src="docs/assets/quest-manager-showcase.png" width="1200" alt="Quest Manager 总览渲染图，展示绿色侧栏、头显插画、存储与电量卡片，以及匿名化的活动列表">
  <br>
  <sub>基于虚构预览数据制作的示意渲染图，设备详情与活动内容以占位条呈现。</sub>
</p>

## 项目文档

- [AGENTS.md](AGENTS.md)：供 agent 使用的阅读顺序、实现边界和工作约定。
- [文档导航](docs/README.md)：英文架构、产品流程、开发、运行和设计决策文档。
- [贡献指南](CONTRIBUTING.md)与[界面设计约定](DESIGN.md)：修改代码和界面时的参考。
- [安全说明](SECURITY.md)与[隐私及数据处理](PRIVACY.md)：ADB、文件操作和本机记录的边界。

## 快速开始

在 Windows x64 的项目根目录打开 64 位 PowerShell。无需预装 Node；脚本会把固定版本的 **Node.js 24.19.0 / npm 11.17.0** 安装到 `env/node`：

```powershell
.\run-install.ps1
.\run-dev.ps1
```

如果当前机器的 PowerShell 执行策略禁止本地脚本，可以只对这次进程指定执行策略：

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run-install.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run-dev.ps1
```

项目内工具安装不需要管理员权限，也不替换系统 Node 或修改永久 PATH。首次运行会下载并校验固定版本的工具链和依赖；首次 Rust 编译需要更多时间。建议为项目预留至少 8 GB 空间；安装系统构建工具还需要额外空间。

Windows 还需要以下系统组件，安装脚本会先检查：

- Visual Studio 2022 / Build Tools（17.x），包含 x64 MSVC 14.30 或以上版本。
- Windows SDK 10.0.19041.0 或以上版本，包含 x64 库、头文件和 `rc.exe`。
- Microsoft Edge WebView2 Runtime 110.0.1531.0 或以上版本。

系统组件已兼容时直接复用；缺失、文件不完整或版本不兼容时，脚本列出要求，并询问是否调用微软官方安装器安装或更新，默认拒绝。C++/SDK 修复会更新选中的 VS 2022 并补齐组件；没有 VS 2022 时安装 Build Tools 2022，保留其他大版本。该操作可能下载数 GB 并请求管理员授权。WebView2 使用 Evergreen 安装器安装或更新。脚本校验微软数字签名、安装后重新检查，不自动重启。也可拒绝后手动安装，见[安装指南](docs/getting-started.md)。

`.\run-install.ps1 -CheckOnly` 只检查系统前置条件，不下载或安装。`-NonInteractive` 安装项目内工具且不询问；系统组件有问题时直接失败，不会静默安装。具体版本边界在 `toolchain.versions.json` 中；版本与文件检查不能代替实际编译验证。

## 功能

- **设备管理**：在 **Devices** 页面添加 USB 或 Wi-Fi 连接，查看两种传输方式，并管理头显设置。两者同时就绪时优先使用 USB，Wi-Fi 作为回退；页面还提供充电时保持唤醒开关和 Lightning Launcher 安装入口。
- **无线连接**：Devices 页面提供 USB 设置、二维码和六位配对码连接方式。
- **Overview**：自动发现设备；按设备序列号合并 USB / 已建立的 Wi-Fi 连接；显示 Android 版本、电量和共享存储容量。
- **Applications**：第三方 / 系统应用列表、名称或包名搜索、图标、版本、APK 大小、启用状态；按推断的 Meta 商店安装 / 侧载等来源筛选，来源不明时保留未知；会话缓存保留筛选、卸载时其他应用的名称和图标；详情包含安装时间、SDK/ABI、权限、split、签名证书和 VR 声明；选择或拖入普通 APK 安装；兼容签名的覆盖更新；导出全部已安装 APK；卸载第三方应用。
- **Files**：浏览共享存储；上传文件或整个目录；下载文件或目录；创建目录、重命名、删除；Downloads、Movies、OBB 快捷入口。
- **安装前预览与编辑**：无需连接头显即可读取本地 APK 的预计名称和图标；连接后对照设备上的安装状态与版本，提示新版、同版、旧版及同批重复包名；可修改显示名称、裁剪替换图标，或开启兼容安装，创建关闭 verity 的重签名副本。
- **Task queue**：后台串行执行变更任务；显示进度、实际错误和固定的目标连接；取消排队任务，以及正在进行的上传、下载和 APK 导出；可清除已结束记录。存在活动任务时，关闭窗口会提示继续等待，也可明确选择退出。

使用前在 Quest 上启用开发者模式。可连接 USB 数据线并在头显中允许 USB 调试；打开 **Devices** 后选择 **Add connection** 进行无线设置。三个页签依次为 **USB setup**、**QR code**、**Pairing code**。USB 设置先检查现有连接和授权，只有需要时才配对，随后自动连接；二维码和配对码也会在配对后验证并连接。电脑与头显需要处于同一局域网；头显深度休眠时 Wi-Fi 会断开，保持无线调试开启时，ADB 可在唤醒后重新连接。配置前需等待活动任务结束；已有 ADB 连接也会自动显示。详见[无线连接流程](docs/product/workflows.md#wireless-setup)。

二维码配对提供两分钟有效期和取消操作。在 Lightning Launcher 中打开 **Android Settings → System → Developer options → Debugging → Wireless debugging → Pair device with QR code** 扫码；可通过 Quest Manager 的 **Lightning Launcher setup** 获取启动器。入口是否可用取决于头显系统；Meta 手机应用的二维码配对属于另一套协议，不能据此判断是否支持 ADB 扫码。缺少该入口时，请使用 USB 设置。

共享存储限定为 `/sdcard`（接受 `/storage/emulated/0` 别名）；具体 `Android/data` / `Android/obb` 的权限取决于系统。普通应用私有目录 `/data/data` 和完整存档备份不在本版能力范围内。

文件上传下载使用临时路径，完成后再改为目标名称；遇到同名目标会报错，不静默覆盖。正常取消或失败时尝试清理临时数据；如果设备断开，任务会给出可能残留的 `.quest-manager-*.partial` 路径。Windows 不支持的文件名、仅大小写不同的路径冲突，以及目录下载中的符号链接会被拒绝。

安装支持普通 `.apk`，暂不支持将 XAPK/APKS/APKM 容器直接安装，也没有批量 split 安装入口。导出 APK 不包含存档。卸载和删除需要界面内确认；正在执行的安装/卸载不能取消。任务记录仅保留当前运行会话，请等待任务完成后再关闭软件。

安装确认页的 **Add OBB files** 可选择多个本地 `.obb` 文件，也可直接拖入确认页，附加到当前选中的 APK。读取不到 APK 包名时禁用该功能。APK 和配套 OBB 作为同一个任务执行，OBB 保留原文件名，放入 `/sdcard/Android/obb/<包名>/`。安装前检查已有文件；内容完全一致时复用，内容不同时拒绝覆盖。APK 安装后逐个传输并校验 OBB；若中途失败，会明确显示“APK 已安装、OBB 数据未完成”，保留已完成文件，可重新选择 APK 和 OBB 手动重试。运行中的整个安装任务不能取消。

**Modify and re-sign APK** 默认关闭。修改名称或图标会重建资源并包含兼容签名处理；只开启 **Compatibility install** 时不修改外观。兼容处理用于部分大 APK 的签名校验溢出，不能解决所有安装问题。原 APK 保持不变；新签名通常不能覆盖原签名版本，也可能影响依赖签名的游戏功能。程序不会自动卸载冲突应用。每个包名使用稳定的本地密钥，请私下备份整个签名目录；详见[隐私说明](PRIVACY.md)。

应用名称与位图图标通过读取 base APK 的部分数据逐步加载，并优先选择可用的英文名称。自适应、矢量或只存在于 split 的图标可能使用默认图标。APK 大小不包括应用数据、缓存或 OBB；证书指纹不验证发布者身份或 APK 完整性，VR 声明也不保证兼容性。

应用元数据在本地缓存。**Clear cached artwork** 清除磁盘缓存并暂停后台读取；**Load app details** 或刷新后恢复。开发模式缓存位于 `env/cache/app-metadata`，发行版位于 `%LOCALAPPDATA%/dev.questmanager.desktop/app-metadata`。缓存与临时资源不应提交，详见[隐私说明](PRIVACY.md)。

## 依赖版本与可复现性

| 项目 | 固定版本 / 记录位置 |
| --- | --- |
| Node.js | 24.19.0，官方 Windows ZIP 与 SHA-256 在 `toolchain.versions.json`；版本同步到 `.node-version` / `package.json` |
| npm | 11.17.0，`packageManager` / `engines` |
| Rust | 1.95.0，`rust-toolchain.toml` |
| Rust target | `x86_64-pc-windows-msvc` |
| rustup | 1.29.0，固定下载地址 + SHA-256 |
| Android Platform-Tools | 37.0.1，固定下载地址 + SHA-256 |
| Android AAPT2 | 9.4.0-15978811（2.20-15978811），固定 Maven 下载地址 + SHA-256 |
| APK 处理工具 | Apktool 3.0.3、Android Build Tools 37.0.0（apksigner/zipalign）、Temurin JRE 21.0.12.1+1；固定下载地址与 SHA-256 |
| APK 编辑 crates | quick-xml 0.42.0、png 0.18.1、getrandom 0.3.4；精确固定 |
| 本地调试二维码编码 | qrcode 0.14.1；禁用默认特性 |
| APK 元数据 crates | zip 4.6.1、sha2 0.10.9、base64 0.22.1；在 `Cargo.toml` 精确固定 |
| Tauri Rust / CLI / JS API | 2.11.5 / 2.11.4 / 2.11.1 |
| HTTPS / reqwest | 0.13.5; rustls |
| React / TypeScript / Vite | 19.3.0 / 7.0.2 / 8.2.2 |
| NSIS / nsis-tauri-utils（可选打包工具） | 3.11 / 0.5.3，由固定的 Tauri CLI 下载并校验，缓存于 `env/target/.tauri` |
| JS 直接与间接依赖 | `package.json` 精确版本 + `package-lock.json` 完整锁定 |
| Rust 直接与间接依赖 | `src-tauri/Cargo.toml` 精确版本 + `src-tauri/Cargo.lock` 完整锁定 |

Tauri 的 Rust crate、CLI、JS API 是独立发布的包，补丁版本不需要一致。安装默认使用 `npm ci` 与 `cargo fetch --locked`，不自动更新锁文件。开发、测试、构建脚本使用同一套项目内环境。

每次安装会生成 `env/installed-versions.json`，记录实际工具版本、系统构建前提和两个锁文件的 SHA-256。构建时复制为 `release/build-environment.json`。这里的可复现性指工具链与依赖解析可复现；系统 SDK、WebView2、链接器版本和签名/打包元数据仍可能影响最终文件，暂不承诺逐字节一致构建。

只有维护者**有意更新依赖**时才使用：

```powershell
# 先修改对应版本清单，再显式重算锁文件，并重新验证。
.\run-install.ps1 -RefreshLocks
```

## Rust 如何管理依赖

Rust 的角色可以对应到 Node 的工具：

| Rust | 类似的 Node 概念 | 在本项目的用途 |
| --- | --- | --- |
| `rustup` | nvm | 安装和选择 Rust 编译器版本 |
| `rustc` | 语言工具链 | 编译 Rust 代码 |
| `cargo` | npm + 构建工具 | 下载 crates、编译、运行和测试 |
| `Cargo.toml` | `package.json` | 声明项目与直接依赖 |
| `Cargo.lock` | `package-lock.json` | 固定所有依赖解析结果 |
| crates.io | npm registry | Rust 包仓库 |

Rust 下载的包源码放在 Cargo 缓存中，实际编译结果放在 `target` 中；不像 Python，它通常不需要在运行时保留一套可导入的源代码环境。本项目通过 `CARGO_HOME`、`RUSTUP_HOME` 和 `CARGO_TARGET_DIR` 把这些内容都收纳进 `env`。

```text
env/
  node/                  固定 Node.js 与随附 npm，按版本子目录存放
  cargo/                 Cargo、rustup 启动器及 crates 下载/源码缓存
  rustup/                固定 Rust 编译器、标准库、rustfmt、Clippy
  node_modules/          npm 安装的全部前端依赖
  npm-cache/             npm 下载缓存
  platform-tools/        adb.exe、所需 DLL、NOTICE 等
  aapt2/                 固定 AAPT2 可执行文件及许可证
  apk-tools/             Apktool、签名和对齐工具、Java 运行时及许可证
  local-data/apk-install/ 私有的 APK 工作副本与持久签名密钥，不可分发
  cache/app-metadata/    开发模式的应用名称、图标与声明缓存
  downloads/             校验过的工具安装包
  target/                Rust 编译缓存、测试程序和构建产物
    .tauri/              NSIS 等可选安装包工具的项目内缓存
  installed-versions.json
node_modules/            指向 env/node_modules 的 Windows junction
```

根目录的 `node_modules` 只是目录联接，便于 Vite、TypeScript 和编辑器按 Node 的常规规则找到包；包实际存放在 `env/node_modules`。项目使用 `env/node` 中的 Node/npm，不依赖系统版本。直接执行 Node/npm 或 Cargo 命令前，先运行 `. .\scripts\Environment.ps1`；环境设置仅影响当前 PowerShell 进程及其子进程，不修改永久 PATH。

保留版本清单、两个锁文件和源代码即可重新安装；`env`、根目录联接、编译输出均已加入 `.gitignore`。不需要提交几 GB 的依赖目录。

## 构建与测试

```powershell
.\run-build.ps1              # 构建 release/quest-manager.exe 和随附 platform-tools、aapt2、apk-tools
.\run-build.ps1 -Installer   # 另生成 NSIS 安装包；首次可能下载打包工具
.\run-build.ps1 -Portable Both # 生成联网引导和内置 WebView2 的离线 portable ZIP
.\run-test.ps1               # 前端构建、Rust 格式、Clippy、单元测试
.\run-test.ps1 -Device       # 加上已授权 Quest 的只读集成测试
.\run-test.ps1 -DeviceWrite  # 加上专用目录 + 专用测试 APK 的完整读写集成测试
```

便携版运行 `release/quest-manager.exe`，分发时保留随附的 `platform-tools`、`aapt2`、`apk-tools` 目录、DLL 和许可证文件，并先检查本机构建记录是否适合分享。不要分发本地签名密钥。安装包输出在 `env/target/release/bundle/nsis`；本地构建暂未配置代码签名。详见[构建与发布说明](docs/development/commit-and-release.md)。

`-DeviceWrite` 只在唯一的 `/sdcard/Download/.quest-manager-test-*` 目录操作，并临时安装不含代码、权限或启动入口的 `dev.questmanager.verification` 测试 APK。测试会验证字节一致、特殊文件名、覆盖拒绝、目录重命名/删除、原版与改名换图标后的安装/更新/导出/卸载，以及签名冲突后原安装仍保留，并清理测试内容及临时测试密钥；如果该测试包预先存在则拒绝执行。固定的测试 APK、源 manifest 和校验值位于 `tests/fixtures`。

仅检查前端界面可以先运行 `. .\scripts\Environment.ps1`，再执行 `npm.cmd run dev`，并在浏览器访问 `http://127.0.0.1:1420/?preview=1`。此模式明确显示 **Preview · sample data**，不操作任何设备。实际使用需要桌面版。

## 项目结构

`-Portable Both` 在 `release/portable-*` 生成两种对比用 ZIP 和 SHA-256 文件，也可用 `Online` 或 `Offline` 单独选择。完整解压后运行 `Start-QuestManager.cmd`：联网版在系统 WebView2 缺失或过旧时询问安装；离线版直接使用包内固定 WebView2。离线指运行环境无需下载，联网功能仍需要网络。两个版本都尚未签名，公开发布前仍需许可证审查和干净 Windows 验证。

关于项目：侧栏 **About** 页面无需连接头显即可查看项目介绍、开发声明、核心依赖版本、源码仓库及许可证全文。连接帮助仍位于 **Help**。

```text
src/                    英文 React 界面、类型、Tauri IPC 和显式预览数据
src-tauri/src/adb.rs     设备查询、应用元数据、目录解析、路径校验
src-tauri/src/metadata.rs 应用详情解析与本地缓存
src-tauri/src/apk.rs     APK 分段读取、资源解析与证书指纹
src-tauri/src/apk_edit.rs APK 名称与图标资源重建
src-tauri/src/apk_install.rs 私有副本、对齐、签名和校验
src-tauri/src/tasks.rs   后台队列、进度、取消、安装/传输/变更任务
src-tauri/src/lib.rs     Tauri IPC 命令与启动
src-tauri/src/device_tests.rs
AGENTS.md               agent 阅读入口与项目约定
docs/                   架构、产品、开发、运行和设计决策
scripts/Environment.ps1 项目内工具链环境
run-install.ps1         安装并检查固定依赖
run-dev.ps1              开发运行
run-build.ps1            生产构建
run-test.ps1             验证入口
```

界面通过明确的业务命令调用 Rust，不接受任意 shell 命令。Rust 使用独立参数启动官方 ADB；远端路径经过共享存储范围校验与 shell 转义。任务以事件向界面同步，并保留查询接口以补足初始状态。应用会复用 ADB 默认服务，不主动执行 `kill-server`。

## 参与贡献与安全报告

GitHub Actions 在拉取请求和推送到 `main` 时执行 Windows 检查，并构建未签名的联网便携版。
推送 `v0.1.0` 这样的版本标签后，会构建联网、离线两个版本，将 ZIP 和 SHA-256 文件附到
Release 草稿，供审查后手动公开发布。触发方式、权限和发布前检查见
[CI 与自动发布构建](docs/development/ci-release.md)。

欢迎提交问题报告、文档改进和代码贡献。请先阅读[贡献指南](CONTRIBUTING.md)、[行为准则](CODE_OF_CONDUCT.md)和[测试说明](docs/development/testing.md)。Issue 可以使用中文或英文；产品界面仍使用英文。仓库包含 Bug、功能建议和 PR 模板。

疑似安全漏洞请按照 [Security](SECURITY.md) 中的私密报告流程处理，不要在公开 Issue 中贴出漏洞细节、个人 APK、设备信息或签名密钥。GitHub 的私密漏洞报告功能需要仓库所有者单独启用。

推送源码与分发二进制是两个检查阶段。发布前请核对[提交与发布清单](docs/development/commit-and-release.md)以及[第三方组件与素材说明](THIRD_PARTY.md)，保留依赖的许可证与声明文件。

## 项目声明与许可证

本程序由 **yexca** 使用 **Codex** 进行开发，开发模型为 **GPT-6-Astra**。

源代码仓库：[github.com/yexca/quest-manager](https://github.com/yexca/quest-manager)。

Copyright © 2026 yexca。本项目使用 [GNU Affero General Public License 第 3 版](LICENSE)（仅限第 3 版，`AGPL-3.0-only`），不提供任何担保。第三方依赖保留各自许可证，项目许可证不替代依赖附带的许可与声明文件。


## 可选的 Lightning Launcher 安装

**Devices** 页面为当前选中的头显提供 **Lightning Launcher** 安装和更新入口，
不依赖 Applications 列表是否已读取完成。

安装窗口按需从作者的 GitHub 获取稳定版 APK，默认选择最新 Launcher。
可选安装 Navigator Button Redirection Service，默认匹配所选 Launcher 源码
明确指定的插件版本，其他历史版本标注兼容性未经确认。保留原始 APK 签名，
通过现有任务队列依次安装；插件需要用户在头显的无障碍设置中启用，程序不会自动授权。
下载限制、部分完成和数据保留见 [产品流程](docs/product/workflows.md#lightning-launcher)
及[隐私说明](PRIVACY.md#optional-app-downloads)。
