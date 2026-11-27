# OpenIDEA 插件

每个插件的源码放在独立目录，`.oiplugin` 安装包位于仓库根目录。

| 插件 | 功能 | 安装包 | 源码 |
| --- | --- | --- | --- |
| Copy Git File Path | 复制当前文件在 Git 远程仓库中的网页链接 | [下载 0.0.3](fxtong.copy-git-file-path-0.0.3.oiplugin?raw=true) | [copy-git-file-path/](copy-git-file-path/) |
| GitLab Merge Requests | 创建 MR、查看项目中打开的 MR、打开或复制 MR 链接 | [下载 0.1.9](fxtong.gitlab-merge-requests-0.1.9.oiplugin?raw=true) | [gitlab-merge-requests/](gitlab-merge-requests/) |

## 安装

1. 从上表下载所需的 `.oiplugin` 文件。
2. 在 OpenIDEA 打开 **Settings → Plugins**，点击齿轮并选择 **Install Plugin from Package File…**。
3. 选择下载的 `.oiplugin` 文件完成安装。

## 插件权限

- Copy Git File Path 使用 `editor.read` 获取当前文件路径，使用 `vcs.read` 读取 Git 根目录、分支和远端地址，再通过 `clipboard` 复制文件链接。
- GitLab Merge Requests 使用 `vcs.read` 读取项目和分支，订阅 `events` 感知 Git 状态变化。它通过 `process:glab` 运行本机已登录的 GitLab CLI，查询项目、分支和 MR，并在确认提交后创建 MR。`clipboard` 用于复制所选 MR 的链接。

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

## 维护者发版流程

### 1. 开发并更新版本

在对应插件目录修改源码，保留 `plugin.toml` 中的插件 ID。同步递增 `plugin.toml`、`Cargo.toml` 和 `Cargo.lock` 中本插件的版本，更新本文顶部的下载链接。功能或权限有变化时，一并更新使用说明和权限用途，并在 OpenIDEA 中验证受影响的菜单或面板操作。

### 2. 构建、签名和校验

准备 Rust 工具链、`wasm32-wasip2` 目标和 OpenIDEA 的 `openidea-plugin` CLI。发布者为 `fxtong`，沿用已有签名私钥。下面的命令从仓库根目录执行，修改第一行即可选择插件。[官方打包工具说明](https://github.com/l0ng-ai/openidea-plugins#publishing-a-plugin)

```sh
plugin=gitlab-merge-requests  # 另一个目录为 copy-git-file-path
(
  set -eu
  root=$(pwd)
  stage=$(mktemp -d /tmp/openidea-release.XXXXXX)
  id=$(awk -F '"' '/^id = / {print $2; exit}' "$plugin/plugin.toml")
  version=$(awk -F '"' '/^version = / {print $2; exit}' "$plugin/plugin.toml")
  wasm=$(printf '%s' "$plugin" | tr '-' '_')
  package="$id-$version.oiplugin"

  rustup target add wasm32-wasip2
  cargo build --manifest-path "$plugin/Cargo.toml" --locked --release \
    --target wasm32-wasip2 --target-dir "$stage/target"
  mkdir -p "$stage/plugin/dist"
  cp "$plugin/plugin.toml" LICENSE README.md "$stage/plugin/"
  cp "$stage/target/wasm32-wasip2/release/$wasm.wasm" "$stage/plugin/dist/plugin.wasm"
  if [ -d "$plugin/icons" ]; then cp -R "$plugin/icons" "$stage/plugin/"; fi

  openidea-plugin pack "$stage/plugin" --publisher fxtong \
    --key "$HOME/.config/openidea/publishers/fxtong.key" --out "$stage/$package"
  openidea-plugin check "$stage/$package"
  cp "$stage/$package" "$root/$package"
  shasum -a 256 "$root/$package"
)
```

新增清单引用的资源时，按清单中的相对路径复制到打包目录。新包校验通过后，删除仓库根目录中该插件的旧版安装包，把源码、版本文件、README 和新包一起提交并推送到 `main`。

### 3. 发布 GitHub Release

在本仓库创建 Release，用新标签指向刚推送的提交，上传根目录中已校验的 `.oiplugin`。从已发布的 Release 下载附件，再用 `shasum -a 256` 与打包时的结果比对。保留历史 Release 的标签和附件，旧版本的市场条目仍会引用它们。[创建 Release](https://github.com/fxtong/openidea_plugins/releases/new)

### 4. 提交官方市场更新

同步注册表 fork 与上游 `l0ng-ai/openidea-plugins` 的 `main`，从最新上游提交建分支。在 `plugins/<插件 ID>.toml` 末尾追加以下条目，保留原有条目。发布者登记沿用 `publishers/fxtong.toml`。[注册表 fork](https://github.com/fxtong/openidea-plugins) · [官方收录规则](https://github.com/l0ng-ai/openidea-plugins#publishing-a-plugin)

```toml
[[versions]]
version = "<与 plugin.toml 一致的新版本>"
package = "https://github.com/fxtong/openidea_plugins/releases/download/<标签>/<插件 ID>-<新版本>.oiplugin"
sha256 = "<Release 附件的 64 位十六进制 SHA-256>"
```

在注册表分支提交条目后，运行 `python3 scripts/registry.py validate --base upstream/main --author fxtong`（`upstream` 指向官方注册表）。推送分支到 fork，向官方注册表的 `main` 提 PR。CI 通过、维护者合并并同步市场索引后，在 OpenIDEA 的插件设置中选择 **Rescan Plugins**，按插件 ID 核对上架版本。
