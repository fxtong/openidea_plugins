# 插件开发与发布

1. 在对应插件目录开发。保留已发布的 `fxtong.*` ID，递增版本，同步 `plugin.toml`、`Cargo.toml` 和 `Cargo.lock` 中本插件的版本，更新 README 下载链接。功能或权限有变化时补充使用说明和权限用途，并验证受影响的行为。
2. 按 [README 的维护者流程](README.md#维护者发版流程) 使用锁定依赖构建 `wasm32-wasip2`，在 `/tmp/` 组装清单、WASM、引用的资源、根目录 LICENSE 和 README。使用发布者 `fxtong` 的现有私钥签名，私钥仅保存在仓库外。
3. 校验新包后，将 `<插件 ID>-<版本>.oiplugin` 放在仓库根目录，删除根目录中该插件的旧包。源码、版本、文档和安装包一起提交。
4. 发布时推送源码提交，在本仓库创建指向该提交的 GitHub Release，上传同一份签名包。从公开附件地址下载并核对 SHA-256。保留历史 Release 的标签和附件，供注册表按原地址取包。
5. 从官方注册表最新 `main` 建分支，在 `plugins/<插件 ID>.toml` 追加 `[[versions]]`，填写版本、Release 附件地址和 SHA-256。保留历史条目，沿用 `publishers/fxtong.toml` 的公钥。推到自己的注册表 fork，向 [OpenIDEA 官方社区插件注册表](https://github.com/l0ng-ai/openidea-plugins) 提 PR。
6. 核对注册表 CI 和维护者合并状态。合并后检查市场索引中的插件 ID 和版本，以索引中可查到新版本作为上架完成的依据。
