# OpenIDEA 插件

每个插件的源码放在独立目录，可单独构建和安装。可直接安装的 `.oiplugin` 文件位于仓库根目录。

| 插件 | 功能 | 源码 |
| --- | --- | --- |
| Copy Git File Path | 复制当前文件在 Git 远程仓库中的网页链接 | [copy-git-file-path/](copy-git-file-path/) |
| GitLab Merge Requests | 创建 MR、查看项目中打开的 MR、打开或复制 MR 链接 | [gitlab-merge-requests/](gitlab-merge-requests/) |

## 构建和安装

需要 Rust 工具链（支持 2024 edition）和支持插件 API `^1.4` 的 OpenIDEA。在仓库根目录执行 `rustup target add wasm32-wasip2`，再按需构建插件。

### Copy Git File Path

```sh
cargo build --manifest-path copy-git-file-path/Cargo.toml --locked --release --target wasm32-wasip2 --target-dir tmp/target
mkdir -p tmp/copy-git-file-path/dist
cp copy-git-file-path/plugin.toml tmp/copy-git-file-path/
cp tmp/target/wasm32-wasip2/release/copy_git_file_path.wasm tmp/copy-git-file-path/dist/plugin.wasm
```

### GitLab Merge Requests

```sh
cargo build --manifest-path gitlab-merge-requests/Cargo.toml --locked --release --target wasm32-wasip2 --target-dir tmp/target
mkdir -p tmp/gitlab-merge-requests/dist tmp/gitlab-merge-requests/icons
cp gitlab-merge-requests/plugin.toml tmp/gitlab-merge-requests/
cp gitlab-merge-requests/icons/mr.svg tmp/gitlab-merge-requests/icons/
cp tmp/target/wasm32-wasip2/release/gitlab_merge_requests.wasm tmp/gitlab-merge-requests/dist/plugin.wasm
```

打开 OpenIDEA 的 **Settings → Plugins → 齿轮菜单 → Install Plugin from Package File…**，选择仓库根目录中对应的 `.oiplugin` 文件。自行构建后，可在同一菜单选 **Install Plugin from Disk…**，再选择 `tmp/copy-git-file-path/` 或 `tmp/gitlab-merge-requests/`。

## 使用方式

### Copy Git File Path

在项目树中右键点击文件，或右键点击编辑器标签页，选择 **Copy Git File Path**。插件根据当前分支、文件相对路径和 `origin` 生成 GitHub 或 GitLab 格式的链接，并复制到剪贴板。

文件需要位于配置了 `origin` 的 Git 仓库中，且当前检出的是分支。链接指向远端分支，打开后看到的是已推送的内容。

### GitLab Merge Requests

先安装 `glab`，用 `glab auth login --hostname <GitLab 主机>` 登录对应实例。在受信任的项目中允许插件运行 `glab`。如果 OpenIDEA 找不到 `glab`，可在插件权限设置中选择可执行文件。

1. 打开配置了 GitLab `origin` 的项目。点击左侧工具栏的 **MR** 图标，或选择 **Tools → GitLab: 新建 Merge Request**。提交前，确保源分支已推到 `origin`，且远端提交与本地 HEAD 一致。
2. 目标分支默认选远端的 `release-<源分支>`，该分支不存在时选 `master`。可以搜索并修改目标分支。标题初始化为 `Merge <源分支> to <初始目标分支>`，切换目标分支后可按需编辑标题。描述、Reviewer 和标签均为选填项，多个 Reviewer 或标签用英文逗号分隔，也可以勾选 **标记为 Draft**。
3. 点击 **提交** 预览，核对项目、分支和表单内容后点 **确认提交**。同一对分支已有打开的 MR 时，插件会定位到已有记录。

**项目 MR** 默认加载当前项目中打开的 MR，单次刷新最多读取 50 条。可通过 **刷新项目 MR** 重新加载，每条记录都可单独点 **在 GitLab 打开** 或 **复制链接**。
