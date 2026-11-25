use openidea_plugin::Project;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Repository {
    pub id: u64,
    pub path_with_namespace: String,
    pub web_url: String,
}

#[derive(Deserialize)]
pub struct MergeRequest {
    pub iid: u64,
    pub title: String,
    pub web_url: String,
    pub source_branch: String,
    pub target_branch: String,
    pub source_project_id: Option<u64>,
    #[serde(default)]
    pub draft: bool,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Draft {
    pub target: String,
    pub title: String,
    pub description: String,
    pub reviewer: String,
    pub labels: String,
    pub draft: bool,
}

#[derive(Deserialize)]
struct Branch {
    commit: Commit,
}

#[derive(Deserialize)]
struct BranchName {
    name: String,
}

#[derive(Deserialize)]
struct Commit {
    id: String,
}

pub fn run(project: &Project, args: &[String]) -> Result<String, String> {
    let output = project.run_process("glab", args, Some("")).map_err(|_| {
        "无法运行 glab。请确认项目受信任，并在 Settings → Plugins 中授权 glab。".to_string()
    })?;
    if output.status == Some(0) {
        return Ok(output.stdout);
    }
    let message = output
        .stderr
        .lines()
        .filter(|line| !line.starts_with("[glab] Update available"))
        .filter(|line| !line.trim().is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");
    if message.is_empty() {
        Err(format!("glab 执行失败，退出状态 {:?}。", output.status))
    } else {
        Err(message)
    }
}

fn api_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

fn project_lookup(origin: &str) -> Result<(String, &str), String> {
    if origin.chars().any(char::is_whitespace) {
        return Err("origin 必须是 GitLab 的 HTTPS 或 SSH 地址。".into());
    }
    let (scheme, host, path) = match origin.split_once("://") {
        Some((scheme @ ("http" | "https" | "ssh"), location)) => {
            let (authority, path) = location
                .split_once('/')
                .ok_or("origin 缺少 GitLab 项目路径。")?;
            if scheme == "ssh" {
                let server = authority.rsplit('@').next().unwrap_or_default();
                let host = if server.starts_with('[') {
                    server
                        .split_once(']')
                        .map(|(address, _)| &server[..address.len() + 1])
                        .unwrap_or(server)
                } else {
                    server.split(':').next().unwrap_or_default()
                };
                ("https", host, path)
            } else {
                if authority.contains('@') {
                    return Err("请移除 origin URL 中的账号或凭据，并通过 glab 登录。".into());
                }
                (scheme, authority, path)
            }
        }
        None => {
            let (server, path) = origin
                .split_once(':')
                .filter(|(server, _)| !server.contains('/'))
                .ok_or("origin 必须是 GitLab 的 HTTPS 或 SSH 地址。")?;
            ("https", server.rsplit('@').next().unwrap_or_default(), path)
        }
        _ => return Err("origin 必须是 GitLab 的 HTTPS 或 SSH 地址。".into()),
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if host.is_empty() || path.is_empty() {
        return Err("origin 缺少 GitLab 主机或项目路径。".into());
    }
    Ok((format!("{scheme}://{host}/{path}"), path))
}

impl Repository {
    pub fn load(project: &Project, origin: &str) -> Result<Self, String> {
        let (lookup_url, origin_path) = project_lookup(origin)?;
        let output = run(
            project,
            &[
                "repo".into(),
                "view".into(),
                "--output=json".into(),
                "--".into(),
                lookup_url.clone(),
            ],
        )
        .map_err(|error| format!("读取 GitLab 项目失败（{lookup_url}）。\n{error}"))?;
        let repo: Self =
            serde_json::from_str(&output).map_err(|_| "无法读取 GitLab 项目信息。".to_string())?;
        if !origin_path.eq_ignore_ascii_case(&repo.path_with_namespace) {
            return Err("origin 路径与 GitLab 当前项目路径不同，请更新 origin 后重试。".into());
        }
        Ok(repo)
    }

    pub fn verify_source(
        &self,
        project: &Project,
        branch: &str,
        commit: &str,
    ) -> Result<(), String> {
        if self.branch(project, branch)? != commit {
            return Err("源分支的远端提交与本地 HEAD 不一致，请先同步分支，再预览。".into());
        }
        Ok(())
    }

    pub fn verify_target(&self, project: &Project, branch: &str) -> Result<(), String> {
        self.branch(project, branch).map(|_| ())
    }

    fn api(&self, project: &Project, path: &str) -> Result<String, String> {
        let host = self
            .web_url
            .split_once("://")
            .and_then(|(_, value)| value.split('/').next())
            .filter(|value| !value.is_empty())
            .ok_or("GitLab 项目地址缺少主机名。")?;
        run(
            project,
            &[
                "api".into(),
                format!("--hostname={host}"),
                format!("projects/{}/{path}", self.id),
            ],
        )
    }

    pub fn branch_names(&self, project: &Project, search: &str) -> Result<Vec<String>, String> {
        let mut path = "repository/branches?per_page=100".to_string();
        if !search.is_empty() {
            path.push_str(&format!("&search={}", api_segment(search)));
        }
        let output = self.api(project, &path)?;
        let branches: Vec<BranchName> =
            serde_json::from_str(&output).map_err(|_| "无法读取 GitLab 分支列表。".to_string())?;
        Ok(branches.into_iter().map(|branch| branch.name).collect())
    }

    pub fn default_target(&self, project: &Project, source: &str) -> Result<String, String> {
        let release = format!("release-{source}");
        match self.branch_commit(project, &release) {
            Ok(_) => Ok(release),
            Err(error) if error.contains("404") => Ok("master".into()),
            Err(error) => Err(format!("无法检查远端分支 {release}。\n{error}")),
        }
    }

    fn branch(&self, project: &Project, branch: &str) -> Result<String, String> {
        self.branch_commit(project, branch).map_err(|error| {
            if error.contains("404") {
                format!("GitLab 找不到远端分支 {branch}。请检查分支名称，或先 Push 源分支。")
            } else {
                error
            }
        })
    }

    fn branch_commit(&self, project: &Project, branch: &str) -> Result<String, String> {
        let path = format!("repository/branches/{}", api_segment(branch));
        let output = self.api(project, &path)?;
        let info: Branch =
            serde_json::from_str(&output).map_err(|_| "无法读取 GitLab 分支信息。".to_string())?;
        Ok(info.commit.id)
    }

    pub fn list(
        &self,
        project: &Project,
        pair: Option<(&str, &str)>,
    ) -> Result<Vec<MergeRequest>, String> {
        let mut args = vec![
            "mr".into(),
            "list".into(),
            format!("--repo={}", self.web_url),
            "--output=json".into(),
            "--per-page=50".into(),
        ];
        if let Some((source, target)) = pair {
            args.extend([
                format!("--source-branch={source}"),
                format!("--target-branch={target}"),
            ]);
        }
        let mut result: Vec<MergeRequest> = serde_json::from_str(&run(project, &args)?)
            .map_err(|_| "无法读取项目 MR 列表。".to_string())?;
        if let Some((source, target)) = pair {
            result.retain(|mr| {
                mr.source_project_id == Some(self.id)
                    && mr.source_branch == source
                    && mr.target_branch == target
            });
        }
        Ok(result)
    }

    pub fn create(&self, project: &Project, source: &str, draft: &Draft) -> Result<String, String> {
        let description = if draft.description == "-" {
            "-\n"
        } else {
            &draft.description
        };
        let mut args = vec![
            "mr".into(),
            "create".into(),
            format!("--repo={}", self.web_url),
            format!("--head={}", self.web_url),
            format!("--source-branch={source}"),
            format!("--target-branch={}", draft.target),
            format!("--title={}", draft.title),
            format!("--description={description}"),
            "--yes".into(),
            "--no-editor".into(),
        ];
        if draft.draft {
            args.push("--draft".into());
        }
        for name in draft
            .reviewer
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            args.push(format!("--reviewer={name}"));
        }
        for label in draft
            .labels
            .split(',')
            .map(str::trim)
            .filter(|label| !label.is_empty())
        {
            args.push(format!("--label={label}"));
        }
        run(project, &args)
    }

    pub fn created_request(
        &self,
        output: &str,
        source: &str,
        draft: &Draft,
    ) -> Option<MergeRequest> {
        let prefix = format!("{}/-/merge_requests/", self.web_url.trim_end_matches('/'));
        let iid = output.split_whitespace().find_map(|part| {
            let suffix = part.strip_prefix(&prefix)?;
            let digits: String = suffix.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u64>().ok().filter(|iid| *iid > 0)
        })?;
        Some(MergeRequest {
            iid,
            title: draft.title.clone(),
            web_url: format!("{prefix}{iid}"),
            source_branch: source.into(),
            target_branch: draft.target.clone(),
            source_project_id: Some(self.id),
            draft: draft.draft,
        })
    }
}
