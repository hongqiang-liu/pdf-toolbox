# pdf_toolbox

`pdf_toolbox` 是一个 Rust + Tauri v2 构建的跨平台 PDF 工具箱。程序启动时会检查命令行参数：存在参数时进入 CLI 模式；没有参数时启动图形界面。

## 功能

- PDF 分割：支持 `1-5,7-12` 页码区间、每 N 页拆分、提取单页。
- PDF 合并：按文件列表顺序合并，支持插入 A4 空白页。
- 文本提取：基于 PDFium 提取标准文字 PDF，扫描图片 PDF 会提示无法提取文字。
- PDF 转图片：基于 PDFium 渲染 PNG/JPG，支持 DPI 和页码范围。

## 目录结构

```text
pdf_toolbox/
├── Cargo.toml
├── package.json
├── README.md
├── src/
│   ├── index.html
│   ├── scripts/app.js
│   └── styles/app.css
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/default.json
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── cli/mod.rs
        ├── core/
        │   ├── error.rs
        │   ├── image_export.rs
        │   ├── merge.rs
        │   ├── mod.rs
        │   ├── progress.rs
        │   ├── split.rs
        │   └── text.rs
        ├── tauri_app/mod.rs
        └── utils/
            ├── fs.rs
            ├── log.rs
            └── mod.rs
```

## 环境要求

- Rust 1.80+
- Node.js 18+
- Windows 10/11 或 macOS 10.15+
- Tauri v2 系统依赖，参考官方文档：https://v2.tauri.app/start/prerequisites/

`pdfium-render` 已启用 `pdfium_latest` feature。构建脚本会从 `bblanchon/pdfium-binaries` 下载当前平台的 PDFium 动态库，缓存到 `src-tauri/pdfium/`，并复制到 `target/debug` 或 `target/release`，所以 `cargo run` 后 CLI 的 `text` / `img` 可直接加载 PDFium。若构建环境不能联网，可手动把 `pdfium.dll` 或 `libpdfium.dylib` 放到可执行文件同目录。

## 安装依赖

```bash
npm install
cargo check
```

## CLI 使用

开发模式下可直接运行 Rust bin：

```bash
cargo run -p pdf_toolbox -- split input.pdf --range 1-10 -o ./output
cargo run -p pdf_toolbox -- split input.pdf --every 5 -o ./output
cargo run -p pdf_toolbox -- split input.pdf --page 3 -o ./output
cargo run -p pdf_toolbox -- merge file1.pdf file2.pdf -o all.pdf
cargo run -p pdf_toolbox -- text book.pdf -o content.txt --page-markers
cargo run -p pdf_toolbox -- img source.pdf --dpi 300 --format png -o ./images
```

打包后示例：

```bash
pdf_toolbox split input.pdf --range 1-10 -o ./output
pdf_toolbox merge file1.pdf file2.pdf -o all.pdf
pdf_toolbox text book.pdf -o content.txt
pdf_toolbox img source.pdf --dpi 300 --format png
```

## GUI 运行

不带任何参数启动即进入 Tauri GUI：

```bash
npm run dev
```

或：

```bash
cargo run -p pdf_toolbox
```

GUI 支持 PDF 拖拽、文件列表、四个功能标签页、输出路径选择、实时进度、日志面板、浅色/深色切换和打开输出目录。

## 打包

```bash
npm run build
```

Windows 会生成安装包和可执行文件；macOS 会生成 `.app`/`.dmg`。具体产物位于：

```text
src-tauri/target/release/
src-tauri/target/release/bundle/
```

## GitHub CI/CD

仓库已内置 GitHub Actions：

- `.github/workflows/ci.yml`：在 `main` / `master` push 和 Pull Request 上运行 Windows、macOS CI。
- `.github/workflows/release.yml`：推送 `v*` tag 时自动创建 draft GitHub Release，并上传 Windows 单 exe、macOS 二进制和 Tauri bundle 产物。
- `.github/dependabot.yml`：每周检查 Cargo、npm、GitHub Actions 依赖更新。

发布新版本：

```bash
git tag v0.1.0
git push origin v0.1.0
```

Release 工作流会生成草稿版本。确认附件和说明无误后，在 GitHub Release 页面手动 Publish。

Windows 单文件产物来自：

```text
target/release/pdf_toolbox.exe
```

这个 exe 已嵌入 PDFium，复制单个文件即可运行 CLI 的 `img` / `text` 功能。GUI 模式仍依赖 Windows WebView2 Runtime。macOS 产物默认未签名；公开分发时建议配置 Apple Developer 证书、公证和 Tauri signing secrets。

## 架构说明

- `src-tauri/src/core/` 是纯 PDF 业务逻辑，不依赖 CLI 或 GUI。
- `src-tauri/src/cli/` 只负责 clap 参数解析和终端输出。
- `src-tauri/src/tauri_app/` 只负责 Tauri IPC、文件选择和进度事件。
- `src-tauri/src/main.rs` 根据命令行参数数量分流到 CLI 或 GUI。
