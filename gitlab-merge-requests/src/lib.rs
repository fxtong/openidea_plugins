mod gitlab;

use gitlab::{Draft, MergeRequest, Repository};
use openidea_plugin::views::{Body, Button, View, ViewEvent, Viewport, Views};
use openidea_plugin::{Error, Event, Invocation, Plugin, Project, host, register_plugin};

const CREATE: &str = "create";
const REQUESTS: &str = "requests";
const TASK_TITLE: &str = "GitLab Merge Requests";
#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum FormMessage {
    Search {
        query: String,
    },
    Preview {
        project: String,
        source: String,
        draft: Draft,
    },
    Submit,
    Back,
}

#[derive(serde::Deserialize)]
struct RequestMessage {
    project: String,
    iid: u64,
    action: RequestAction,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestAction {
    Open,
    Copy,
}

#[derive(PartialEq, Eq)]
struct Checkout {
    root: String,
    origin: String,
    branch: String,
    commit: String,
}

impl Checkout {
    fn read(project: &Project) -> Result<Self, String> {
        let status = project
            .vcs_status()
            .map_err(|error| error.to_string())?
            .ok_or("请先打开 Git 项目。")?;
        let branch = status
            .branch
            .ok_or("当前是 detached HEAD，请先切换到源分支。")?;
        let commit = status.commit.ok_or("当前分支还没有提交。")?;
        let origin = project
            .vcs_remotes()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|(name, _)| name == "origin")
            .map(|(_, url)| url)
            .ok_or("当前 Git 项目没有配置 origin。")?;
        Ok(Self {
            root: status.root,
            origin,
            branch,
            commit,
        })
    }
}

#[derive(Default)]
struct Main {
    checkout: Option<Checkout>,
    repo: Option<Repository>,
    draft: Draft,
    form_initialized: bool,
    target_options: Vec<String>,
    preview: bool,
    status: String,
    list_status: String,
    list_loaded: bool,
    requests: Vec<MergeRequest>,
    highlighted_iid: Option<u64>,
}

impl Main {
    fn sync_checkout(&mut self, project: &Project) -> Result<(), String> {
        let checkout = Checkout::read(project)?;
        if self.checkout.as_ref() != Some(&checkout) {
            let project_changed = self
                .checkout
                .as_ref()
                .is_none_or(|saved| saved.root != checkout.root || saved.origin != checkout.origin);
            if project_changed {
                *self = Self::default();
            } else {
                if self
                    .checkout
                    .as_ref()
                    .is_some_and(|saved| saved.branch != checkout.branch)
                {
                    self.draft = Draft::default();
                    self.form_initialized = false;
                    self.target_options.clear();
                }
                self.preview = false;
                self.status.clear();
            }
            self.checkout = Some(checkout);
        }
        Ok(())
    }

    fn load_repo(&mut self, project: &Project) -> Result<(), String> {
        let checkout = self.checkout.as_ref().ok_or("请先打开 Git 项目。")?;
        self.repo = Some(Repository::load(project, &checkout.origin)?);
        Ok(())
    }

    fn initialize_form(&mut self, project: &Project) -> Result<(), String> {
        let checkout = self.checkout.as_ref().ok_or("请先打开 Git 项目。")?;
        let repo = self.repo.as_ref().ok_or("请先读取 GitLab 项目。")?;
        let target = repo.default_target(project, &checkout.branch)?;
        self.target_options = repo.branch_names(project, "").unwrap_or_default();
        self.draft.title = format!("Merge {} to {target}", checkout.branch);
        self.draft.target = target;
        self.form_initialized = true;
        Ok(())
    }

    fn refresh_requests(&mut self, project: &Project) -> Result<(), String> {
        self.list_loaded = true;
        let repo = self.repo.as_ref().ok_or("请先读取 GitLab 项目。")?;
        self.requests = repo.list(project, None)?;
        self.list_status = format!("{} 条未关闭 MR（最多显示 50 条）", self.requests.len());
        Ok(())
    }

    fn show_request(&mut self, project: &Project, request: MergeRequest, status: &str) {
        self.highlighted_iid = Some(request.iid);
        let list = self.refresh_requests(project);
        self.requests.retain(|entry| entry.iid != request.iid);
        self.requests.insert(0, request);
        self.status = status.into();
        self.list_status = match list {
            Ok(()) => status.into(),
            Err(error) => format!("{status}。刷新列表失败：{error}"),
        };
        self.preview = false;
        host::refresh_view(REQUESTS);
        let _ = host::reveal_view(REQUESTS);
    }

    fn prepare(&mut self, project: &Project, submit: bool) -> Result<(), String> {
        let checkout = self.checkout.as_ref().ok_or("请先打开 Git 项目。")?;
        let repo = self.repo.as_ref().ok_or("请先读取 GitLab 项目。")?;
        self.draft.target = self.draft.target.trim().to_string();
        self.draft.title = self.draft.title.trim().to_string();
        if self.draft.target.is_empty() || self.draft.title.is_empty() {
            return Err("请填写目标分支和标题。".into());
        }
        if self.draft.target == checkout.branch {
            return Err("源分支与目标分支相同。".into());
        }
        let pair = Some((checkout.branch.as_str(), self.draft.target.as_str()));
        if let Some(request) = repo.list(project, pair)?.into_iter().next() {
            let status = format!("同一对分支已有 MR !{}", request.iid);
            self.show_request(project, request, &status);
            return Ok(());
        }
        repo.verify_source(project, &checkout.branch, &checkout.commit)?;
        if !submit {
            repo.verify_target(project, &self.draft.target)?;
            self.preview = true;
            self.status.clear();
            return Ok(());
        }
        let creation = repo.create(project, &checkout.branch, &self.draft);
        let exact = creation
            .as_ref()
            .ok()
            .and_then(|output| repo.created_request(output, &checkout.branch, &self.draft))
            .or_else(|| {
                repo.list(project, pair)
                    .ok()
                    .and_then(|requests| requests.into_iter().next())
            });
        match (creation, exact) {
            (Ok(_), Some(request)) => {
                let status = format!("已创建 MR !{}", request.iid);
                self.draft = Draft::default();
                self.form_initialized = false;
                self.show_request(project, request, &status);
            }
            (Ok(_), None) => {
                self.preview = false;
                self.status = "GitLab 已接受创建。请刷新项目 MR 确认记录。".into();
                self.list_loaded = false;
            }
            (Err(_), Some(request)) => {
                let status = format!("已找到同分支的 MR !{}", request.iid);
                self.show_request(project, request, &status);
            }
            (Err(error), _) => {
                return Err(format!(
                    "创建失败或结果未知，请核对项目 MR 后再试。\n{error}"
                ));
            }
        }
        Ok(())
    }

    fn create_view(&self, checkout: &Checkout, repo: &Repository) -> View {
        let data = serde_json::json!({
            "project": repo.web_url,
            "source": checkout.branch,
            "draft": self.draft,
            "branches": self.target_options,
            "preview": self.preview,
            "status": self.status,
        })
        .to_string()
        .replace('<', "\\u003c");
        View::new(Body::Html(
            include_str!("form.html").replace("<!--FORM_STATE-->", &data),
        ))
    }

    fn apply_form(&mut self, project: &Project, message: FormMessage) -> Result<(), String> {
        match message {
            FormMessage::Search { query } => {
                if !self.form_initialized {
                    return Ok(());
                }
                let repo = self.repo.as_ref().ok_or("请先读取 GitLab 项目。")?;
                let result = repo.branch_names(project, query.trim());
                let response = match result {
                    Ok(branches) => serde_json::json!({ "query": query, "branches": branches }),
                    Err(error) => serde_json::json!({ "query": query, "error": error }),
                };
                host::post_view_message(CREATE, &response.to_string())
                    .map_err(|error| error.to_string())
            }
            FormMessage::Preview {
                project: shown_project,
                source,
                draft,
            } => {
                let repo = self.repo.as_ref().ok_or("项目已切换，请核对表单。")?;
                let checkout = self.checkout.as_ref().ok_or("请先打开 Git 项目。")?;
                if repo.web_url != shown_project
                    || checkout.branch != source
                    || !self.form_initialized
                {
                    return Err("当前项目或源分支已变更，请核对后重新提交。".into());
                }
                self.draft = draft;
                self.prepare(project, false)
            }
            FormMessage::Submit if self.preview => self.prepare(project, true),
            FormMessage::Submit => Err("当前项目、分支或提交已变更，请重新预览。".into()),
            FormMessage::Back => {
                self.preview = false;
                self.status.clear();
                Ok(())
            }
        }
    }

    fn list_view(&self, repo: &Repository) -> View {
        let requests: Vec<_> = self
            .requests
            .iter()
            .map(|mr| {
                serde_json::json!({
                    "iid": mr.iid,
                    "title": mr.title,
                    "source": mr.source_branch,
                    "target": mr.target_branch,
                    "draft": mr.draft,
                })
            })
            .collect();
        let data = serde_json::json!({
            "project": repo.web_url,
            "status": self.list_status,
            "requests": requests,
            "highlighted": self.highlighted_iid,
        })
        .to_string()
        .replace('<', "\\u003c");
        View::new(Body::Html(
            include_str!("requests.html").replace("<!--REQUESTS_STATE-->", &data),
        ))
        .button(Button::new("refresh", "刷新项目 MR").icon("refresh"))
    }

    fn apply_request(&self, message: &RequestMessage) -> Result<(), String> {
        let repo = self.repo.as_ref().ok_or("请先读取 GitLab 项目。")?;
        if repo.web_url != message.project {
            return Err("当前项目已切换，请刷新列表。".into());
        }
        let request = self
            .requests
            .iter()
            .find(|mr| mr.iid == message.iid)
            .ok_or("找不到这条 MR，请刷新列表。")?;
        match message.action {
            RequestAction::Open => host::open_url(&request.web_url),
            RequestAction::Copy => host::copy_to_clipboard(&request.web_url),
        }
        .map_err(|error| error.to_string())
    }
}

impl Plugin for Main {
    fn run_command(
        &mut self,
        id: &str,
        _ctx: &Invocation,
        _arg: Option<&str>,
    ) -> Result<Option<String>, Error> {
        if id == "new" {
            host::reveal_view(CREATE)?;
        }
        Ok(None)
    }

    fn handle_events(&mut self, events: &[Event], ctx: &Invocation) {
        if !events.iter().any(|event| matches!(event, Event::VcsChanged(_))) {
            return;
        }
        let Some(project) = ctx.project() else {
            return;
        };
        if Checkout::read(&project).ok().as_ref() == self.checkout.as_ref() {
            return;
        }
        host::refresh_view(CREATE);
        host::refresh_view(REQUESTS);
    }
}

impl Views for Main {
    fn render(
        &mut self,
        view: &str,
        _viewport: Option<Viewport>,
        ctx: &Invocation,
    ) -> Result<View, Error> {
        let Some(project) = ctx.project() else {
            return Ok(View::message("请先打开 Git 项目。"));
        };
        if let Err(message) = self.sync_checkout(&project) {
            return Ok(View::message(&message));
        }
        let _task = (self.repo.is_none() || !self.form_initialized || !self.list_loaded)
            .then(|| host::begin_task(TASK_TITLE, false));
        if self.repo.is_none() && self.status.is_empty() {
            if let Err(message) = self.load_repo(&project) {
                self.status = message;
            }
        }
        if self.repo.is_none() {
            return Ok(
                View::message(&self.status).button(Button::new("reload", "重新读取 GitLab 项目"))
            );
        }
        if view == CREATE {
            if !self.form_initialized {
                if let Err(message) = self.initialize_form(&project) {
                    return Ok(View::message(&message)
                        .button(Button::new("reload", "重新读取 GitLab 项目")));
                }
            }
            return Ok(
                self.create_view(self.checkout.as_ref().unwrap(), self.repo.as_ref().unwrap())
            );
        }
        if !self.list_loaded {
            if let Err(message) = self.refresh_requests(&project) {
                self.list_status = message;
            }
        }
        Ok(self.list_view(self.repo.as_ref().unwrap()))
    }

    fn handle(&mut self, view: &str, event: ViewEvent, ctx: &Invocation) -> Result<(), Error> {
        let Some(project) = ctx.project() else {
            return Ok(());
        };
        if let Err(message) = self.sync_checkout(&project) {
            host::refresh_view(view);
            return Err(message.into());
        }
        let result = match event {
            ViewEvent::Message(raw) if view == CREATE => {
                let Ok(message) = serde_json::from_str::<FormMessage>(&raw) else {
                    return Ok(());
                };
                if matches!(&message, FormMessage::Search { .. }) && self.form_initialized {
                    return self.apply_form(&project, message).map_err(Into::into);
                }
                let _task = host::begin_task(TASK_TITLE, false);
                let result = self.apply_form(&project, message);
                let _ = host::post_view_message(CREATE, r#"{"complete":true}"#);
                result
            }
            ViewEvent::Message(raw) if view == REQUESTS => {
                let Ok(message) = serde_json::from_str::<RequestMessage>(&raw) else {
                    return Ok(());
                };
                let feedback = match self.apply_request(&message) {
                    Ok(()) => match message.action {
                        RequestAction::Open => "",
                        RequestAction::Copy => "已复制链接",
                    }
                    .to_string(),
                    Err(error) => error,
                };
                let response = serde_json::json!({ "iid": message.iid, "message": feedback });
                return host::post_view_message(REQUESTS, &response.to_string());
            }
            ViewEvent::Button(id) => {
                let _task = host::begin_task(TASK_TITLE, false);
                match id.as_str() {
                    "reload" => {
                        self.status.clear();
                        self.load_repo(&project)
                    }
                    "refresh" if view == REQUESTS => self.refresh_requests(&project),
                    _ => Ok(()),
                }
            }
            _ => return Ok(()),
        };
        if let Err(message) = result {
            if view == CREATE || self.repo.is_none() {
                self.status = message;
            } else {
                self.list_status = message;
            }
        }
        host::refresh_view(view);
        Ok(())
    }
}

register_plugin!(Main, views);
