use openidea_plugin::{Error, Invocation, NotificationLevel, Plugin, host, register_plugin};

const COPY_FAILED: &str = "复制失败：请确认文件位于 Git 仓库内，并且存在 origin 远程仓库";

#[derive(Default)]
pub struct Main;

impl Plugin for Main {
    fn run_command(&mut self, _id: &str, ctx: &Invocation, _arg: Option<&str>) -> Result<Option<String>, Error> {
        let file_path = match ctx.selected_paths().into_iter().next() {
            Some(path) => path,
            None => ctx.editor().map(|editor| editor.path()).ok_or("请选择一个本地文件")?,
        };

        let project = ctx.project().ok_or(COPY_FAILED)?;
        let status = project.vcs_status()?.ok_or(COPY_FAILED)?;
        let git_root = status.root;
        let branch = status.branch.ok_or(COPY_FAILED)?;
        let mut remote = project
            .vcs_remotes()?
            .into_iter()
            .find(|(name, _)| name == "origin")
            .map(|(_, url)| url)
            .ok_or(COPY_FAILED)?;

        // scp-like (`user@host:path`) -> https://host/path
        let scp_like = remote.find('@').filter(|at| *at > 0).and_then(|at| {
            let after = &remote[at + 1..];
            after.find(':').filter(|colon| *colon > 0).map(|colon| format!("https://{}/{}", &after[..colon], &after[colon + 1..]))
        });
        if let Some(https) = scp_like {
            remote = https;
        } else if let Some(rest) = remote.strip_prefix("ssh://git@") {
            remote = format!("https://{rest}");
        }
        let remote = remote.strip_suffix(".git").unwrap_or(&remote).to_string();

        // Same unreserved set as JavaScript's encodeURIComponent.
        let encode = |segment: &str| -> String {
            segment
                .bytes()
                .map(|byte| match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => (byte as char).to_string(),
                    _ => format!("%{byte:02X}"),
                })
                .collect()
        };

        let relative = file_path
            .strip_prefix(git_root.trim_end_matches('/'))
            .ok_or(COPY_FAILED)?
            .trim_start_matches('/');
        let relative_path = relative.split('/').map(&encode).collect::<Vec<_>>().join("/");
        let branch_path = branch.split('/').map(&encode).collect::<Vec<_>>().join("/");

        let is_gitlab = remote.to_lowercase().contains("gitlab") || remote.contains("git.garena.com");
        let url = if is_gitlab {
            format!("{remote}/-/blob/{branch_path}/{relative_path}")
        } else {
            format!("{remote}/blob/{branch_path}/{relative_path}")
        };

        host::copy_to_clipboard(&url).map_err(|_| COPY_FAILED)?;
        host::notify(NotificationLevel::Info, "Copy Git File Path", &format!("已复制 Git 文件地址：{url}"));
        Ok(Some(url))
    }
}

register_plugin!(Main);
