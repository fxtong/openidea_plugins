# OpenIDEA 插件

每个插件放在独立目录，可单独构建和安装。

| 插件 | 功能 | 源码 |
| --- | --- | --- |
| Copy Git File Path | 复制当前文件在 Git 远程仓库中的网页链接 | [copy-git-file-path/](copy-git-file-path/) |

## Copy Git File Path

在项目树中右键点击文件，或右键点击编辑器标签页，选择 **Copy Git File Path**。插件根据当前分支、文件相对路径和 `origin` 生成 GitHub 或 GitLab 格式的链接，并复制到剪贴板。

文件需要位于配置了 `origin` 的 Git 仓库中。分支和文件推送到远端后，才能通过生成的链接查看对应内容。

### 本地安装

需要 Rust 工具链和支持插件 API `^1.4` 的 OpenIDEA。在仓库根目录运行：

```sh
cd copy-git-file-path
rustup target add wasm32-wasip2
cargo build --locked --release --target wasm32-wasip2
mkdir -p dist
cp target/wasm32-wasip2/release/copy_git_file_path.wasm dist/plugin.wasm
```

在 OpenIDEA 中打开 **Settings → Plugins → Install Plugin from Disk…**，选择 `copy-git-file-path/` 目录并确认安装。
