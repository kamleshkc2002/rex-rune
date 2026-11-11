//! Settings → Integrations: Rex's skill and session hooks for coding agents on this machine,
//! managed with the local `rex skill` command. Nothing runs until the section is opened or an
//! action is chosen, every change is previewed with `--dry-run` first, and every invocation
//! passes `--autostart=false` so it can never start a server.
use super::{
    panel::PanelKind,
    settings_kit::{
        self, CONTROL, ChipTone, INSET, card, footnote, mono_footnote, page, row_frame, setting,
        state_chip,
    },
    theme::*,
    widgets::{Pill, glyph, pill},
    workspace::Workspace,
};
use rune::{El, Tx, col, row, text};
use rune_kit::Switch;
use serde::Deserialize;

/// The agents Settings knows, in the order it lists them.
///
/// OMP reads the same Agent Skills directory as Pi. Rex does not yet accept an `omp`
/// harness name, so changes for OMP use Rex's Pi target to manage that shared file.
pub const AGENTS: [(&str, &str); 5] = [
    ("claude", "Claude Code"),
    ("codex", "Codex"),
    ("opencode", "OpenCode"),
    ("pi", "Pi"),
    ("omp", "OMP"),
];

fn agent_name(id: &str) -> &str {
    AGENTS
        .iter()
        .find(|(a, _)| *a == id)
        .map_or(id, |(_, name)| name)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Item {
    pub kind: String,
    #[serde(default)]
    pub path: Option<String>,
    pub state: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Harness {
    pub id: String,
    #[serde(default)]
    pub detected: bool,
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct StatusReply {
    #[serde(default)]
    harnesses: Vec<Harness>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Step {
    #[serde(default)]
    pub harnesses: Vec<String>,
    pub kind: String,
    #[serde(default)]
    pub path: Option<String>,
    pub change: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ChangeReply {
    #[serde(default)]
    steps: Vec<Step>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AgentStatus {
    #[default]
    Unknown,
    Checking,
    Ready(Vec<Harness>),
    /// `rex` is missing or failed; the words say which.
    Unavailable(String),
}

/// One agent's install state, folded from its skill and hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Installed {
    Yes,
    Outdated,
    No,
    Conflict,
    Unreadable,
}

impl Harness {
    pub fn installed(&self) -> Installed {
        let items: Vec<_> = self
            .items
            .iter()
            .filter(|i| i.state != "unsupported")
            .collect();
        let any = |s: &str| items.iter().any(|i| i.state == s);
        if any("foreign") {
            Installed::Conflict
        } else if any("unreadable") {
            Installed::Unreadable
        } else if !items.is_empty() && items.iter().all(|i| i.state == "current") {
            Installed::Yes
        } else if items.iter().all(|i| i.state == "missing") {
            Installed::No
        } else {
            Installed::Outdated
        }
    }

    fn has_rex_files(&self) -> bool {
        self.items
            .iter()
            .any(|i| i.state == "current" || i.state == "outdated")
    }

    fn skill_path(&self) -> Option<&str> {
        self.items
            .iter()
            .find(|i| i.kind == "skill")
            .and_then(|i| i.path.as_deref())
    }
}

/// What the user asked to change. No agents means every agent `rex` detects.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentChange {
    Setup { agents: Vec<String>, force: bool },
    Remove { agents: Vec<String> },
}

impl AgentChange {
    /// The `rex` arguments, with the session-naming choice always explicit: leaving it out
    /// would keep whatever an earlier setup installed.
    pub fn args(&self, naming: bool, dry_run: bool) -> Vec<String> {
        let mut args: Vec<String> = vec!["--autostart=false".into(), "skill".into()];
        let agents = match self {
            AgentChange::Setup { agents, .. } => {
                args.push("setup".into());
                agents
            }
            AgentChange::Remove { agents } => {
                args.push("remove".into());
                agents
            }
        };
        let mut selected = Vec::new();
        for requested in agents {
            // Rex has no `omp` agent; remove this mapping when it does.
            let agent = if requested == "omp" {
                "pi"
            } else {
                requested.as_str()
            };
            if !selected.contains(&agent) {
                selected.push(agent);
                args.push(format!("--agent={agent}"));
            }
        }
        if let AgentChange::Setup { force, .. } = self {
            args.push(format!("--session-naming={naming}"));
            if *force {
                args.push("--force".into());
            }
        }
        if dry_run {
            args.push("--dry-run".into());
        }
        args.push("--json".into());
        args
    }

    fn agents_text(&self) -> String {
        let agents = match self {
            AgentChange::Setup { agents, .. } | AgentChange::Remove { agents } => agents,
        };
        if agents.is_empty() {
            return "your agents".into();
        }
        let names: Vec<_> = agents.iter().map(|a| agent_name(a)).collect();
        names.join(", ")
    }
}

pub fn status_args() -> Vec<String> {
    ["--autostart=false", "skill", "status", "--json"]
        .map(String::from)
        .to_vec()
}

/// The environment OMP is detected from.
#[derive(Clone, Debug, Default)]
pub struct OmpEnv {
    pub home: Option<std::path::PathBuf>,
    pub path: Option<std::ffi::OsString>,
    pub pi_coding_agent_dir: Option<std::path::PathBuf>,
}

impl OmpEnv {
    pub fn current() -> Self {
        OmpEnv {
            home: std::env::var_os("HOME").map(Into::into),
            path: std::env::var_os("PATH"),
            pi_coding_agent_dir: std::env::var_os("PI_CODING_AGENT_DIR").map(Into::into),
        }
    }

    /// OMP's agent directory exists (`$PI_CODING_AGENT_DIR` or `~/.omp/agent`), or `omp` is
    /// on `PATH`.
    pub fn detected(&self) -> bool {
        let configured = self
            .pi_coding_agent_dir
            .as_ref()
            .is_some_and(|dir| dir.is_dir());
        let default = self
            .home
            .as_ref()
            .is_some_and(|home| home.join(".omp/agent").is_dir());
        let on_path = self.path.as_ref().is_some_and(|path| {
            std::env::split_paths(path).any(|directory| directory.join("omp").is_file())
        });
        configured || default || on_path
    }
}

/// OMP discovers the Agent Skills path that Rex already manages for Pi. Mirror that
/// file's state so OMP gets a first-class row without duplicating Rex's ownership logic.
fn add_omp_harness(harnesses: &mut Vec<Harness>, omp: &OmpEnv) {
    if harnesses.iter().any(|harness| harness.id == "omp") {
        return;
    }
    let items = harnesses
        .iter()
        .find(|harness| harness.id == "pi")
        .and_then(|harness| {
            harness
                .items
                .iter()
                .find(|item| item.kind == "skill")
                .cloned()
        })
        .into_iter()
        .collect();
    harnesses.push(Harness {
        id: "omp".into(),
        detected: omp.detected(),
        items,
    });
}

pub struct Integrations {
    pub status: AgentStatus,
    /// A preview or change is running; actions wait for it.
    pub busy: bool,
    /// `rex` invocations headless tests would have made.
    pub ran: Vec<Vec<String>>,
    pub omp: OmpEnv,
}

impl Default for Integrations {
    fn default() -> Self {
        Integrations {
            status: AgentStatus::default(),
            busy: false,
            ran: Vec::new(),
            omp: if cfg!(test) {
                OmpEnv::default()
            } else {
                OmpEnv::current()
            },
        }
    }
}

fn local_rex() -> Option<std::path::PathBuf> {
    let directory = std::env::var_os("XDG_BIN_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".local/bin"))
        })?;
    let path = directory.join("rex");
    path.is_file().then_some(path)
}

fn rex_output(program: &std::ffi::OsStr, args: &[String]) -> std::io::Result<std::process::Output> {
    std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
}

/// Run `rex` with `args` and return its standard output, or words for the user.
fn run_rex(args: &[String]) -> Result<String, String> {
    let output = match rex_output(std::ffi::OsStr::new("rex"), args) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match local_rex() {
            Some(path) => rex_output(path.as_os_str(), args),
            None => Err(error),
        },
        result => result,
    }
    .map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "The rex command isn't installed on this machine (or isn't on PATH or in the user bin directory), so agents can't be set up from here.".to_owned()
        } else {
            format!("Couldn't run rex: {error}")
        }
    })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        Err(format!("rex skill failed: {}", reason.trim()))
    }
}

fn home_relative(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && path.starts_with(&home) => {
            format!("~{}", &path[home.len()..])
        }
        _ => path.to_owned(),
    }
}

impl Workspace {
    /// Run `rex` off the UI thread and hand its result to `done` on it.
    fn spawn_rex(
        &mut self,
        args: Vec<String>,
        done: impl FnOnce(&mut Workspace, Result<String, String>, &mut Tx) + Send + 'static,
    ) {
        if cfg!(test) {
            self.integrations.ran.push(args);
            return;
        }
        let handle = self.handle.clone();
        let _ = std::thread::Builder::new()
            .name("rex-skill".into())
            .spawn(move || {
                let result = run_rex(&args);
                handle.update(move |s, tx| done(s, result, tx));
            });
    }

    /// Ask `rex skill status` what's installed.
    pub fn refresh_integrations(&mut self) {
        if self.integrations.status == AgentStatus::Checking {
            return;
        }
        self.integrations.status = AgentStatus::Checking;
        self.spawn_rex(status_args(), |s, result, _| s.apply_agent_status(result));
    }

    pub fn apply_agent_status(&mut self, result: Result<String, String>) {
        self.integrations.status = match result.and_then(|out| {
            serde_json::from_str::<StatusReply>(&out)
                .map_err(|e| format!("Couldn't read rex skill status: {e}"))
        }) {
            Ok(mut reply) => {
                add_omp_harness(&mut reply.harnesses, &self.integrations.omp);
                AgentStatus::Ready(reply.harnesses)
            }
            Err(words) => AgentStatus::Unavailable(words),
        };
    }

    /// Preview a change: a dry run whose steps the confirmation lists.
    pub fn preview_agents(&mut self, change: AgentChange) {
        if self.integrations.busy {
            return;
        }
        self.integrations.busy = true;
        let args = change.args(self.settings.agent_session_naming, true);
        self.spawn_rex(args, move |s, result, _| {
            s.show_agent_preview(change, result)
        });
    }

    pub fn show_agent_preview(&mut self, change: AgentChange, result: Result<String, String>) {
        self.integrations.busy = false;
        let reply = result.and_then(|out| {
            serde_json::from_str::<ChangeReply>(&out)
                .map_err(|e| format!("Couldn't read what rex would change: {e}"))
        });
        match reply {
            Ok(reply) => {
                let title = match &change {
                    AgentChange::Setup { force: true, .. } => {
                        format!("Replace the rex skill for {}?", change.agents_text())
                    }
                    AgentChange::Setup { .. } => {
                        format!("Set up Rex for {}?", change.agents_text())
                    }
                    AgentChange::Remove { .. } => {
                        format!("Remove Rex from {}?", change.agents_text())
                    }
                };
                self.open_panel(
                    PanelKind::AgentChange {
                        change,
                        steps: reply.steps,
                    },
                    &title,
                    &[],
                );
            }
            Err(words) => self.notify(words),
        }
    }

    /// The confirmed change: run it for real, then read the status again.
    pub fn apply_agents(&mut self, change: AgentChange) {
        self.integrations.busy = true;
        let args = change.args(self.settings.agent_session_naming, false);
        self.spawn_rex(args, move |s, result, _| {
            s.integrations.busy = false;
            match result {
                Ok(_) => s.inform(match &change {
                    AgentChange::Setup { .. } => {
                        format!("Rex is set up for {}.", change.agents_text())
                    }
                    AgentChange::Remove { .. } => {
                        format!("Removed Rex from {}.", change.agents_text())
                    }
                }),
                Err(words) => s.notify(words),
            }
            s.integrations.status = AgentStatus::Unknown;
            s.refresh_integrations();
        });
    }

    /// Flip "Let agents name sessions"; agents already set up get the new choice after a
    /// preview.
    pub fn toggle_session_naming(&mut self) {
        self.settings.agent_session_naming = !self.settings.agent_session_naming;
        self.save();
        if let AgentStatus::Ready(harnesses) = &self.integrations.status {
            let agents: Vec<String> = harnesses
                .iter()
                .filter(|h| h.has_rex_files())
                .map(|h| h.id.clone())
                .collect();
            if !agents.is_empty() {
                self.preview_agents(AgentChange::Setup {
                    agents,
                    force: false,
                });
            }
        }
    }

    /// Other agents sharing `harness`'s skill directory, which a change to it also changes.
    fn sharing(&self, harness: &str) -> Vec<&'static str> {
        let AgentStatus::Ready(all) = &self.integrations.status else {
            return Vec::new();
        };
        let Some(path) = all
            .iter()
            .find(|h| h.id == harness)
            .and_then(|h| h.skill_path())
        else {
            return Vec::new();
        };
        all.iter()
            .filter(|h| h.id != harness && h.skill_path() == Some(path))
            .filter_map(|h| AGENTS.iter().find(|(id, _)| *id == h.id).map(|a| a.1))
            .collect()
    }
}

/// A button that waits for a running `rex` before it can be pressed again.
fn wait(e: El, busy: bool) -> El {
    if busy { e.opacity(0.45).inert() } else { e }
}

/// The agent names' column, so every row's state chip starts at one x.
const NAME_COLUMN: f32 = 104.;

/// What an agent's chip says, how it reads, and the sentence behind the word when the word
/// alone doesn't say what is wrong.
pub fn agent_state(harness: &Harness) -> (&'static str, ChipTone, &'static str) {
    match harness.installed() {
        Installed::Yes => ("Installed", ChipTone::Good, ""),
        Installed::Outdated => ("Update available", ChipTone::Attention, ""),
        Installed::No if !harness.detected => {
            ("Not found", ChipTone::Off, "Not found on this machine")
        }
        Installed::No => ("Not installed", ChipTone::Off, ""),
        Installed::Conflict => (
            "Conflict",
            ChipTone::Attention,
            "Another skill named rex is installed",
        ),
        Installed::Unreadable => (
            "Unreadable",
            ChipTone::Attention,
            "Rex can't read this agent's settings",
        ),
    }
}

/// One agent: its mark and name, a chip for its state and the actions that state allows.
fn agent_row(s: &Workspace, harness: &Harness) -> El {
    let id = harness.id.clone();
    let installed = harness.installed();
    let (state, tone, why) = agent_state(harness);
    // An agent that isn't on this machine steps back; there is nothing to do for it.
    let absent = installed == Installed::No && !harness.detected;
    let name = agent_name(&id).to_owned();
    let mut line = row()
        .gap(10.)
        .center_y()
        .grow(1.)
        .min_w(0.)
        .child(glyph(
            super::activity::program_icon(Some(&id)),
            16.,
            rgb(muted()),
        ))
        .child(
            row().w(NAME_COLUMN).child(
                text(name.clone())
                    .font(ui(13.))
                    .color(rgb(if absent { muted() } else { ink() }))
                    .truncate()
                    .help_when_truncated(name),
            ),
        )
        .child(state_chip(
            &format!("agent-state:{id}"),
            state,
            tone,
            if tone == ChipTone::Attention { "" } else { why },
        ));
    // Trouble says what is wrong beside its word, not only in a tooltip.
    if tone == ChipTone::Attention && !why.is_empty() {
        line = line.child(
            text(why)
                .font(ui(11.5))
                .color(rgb(muted()))
                .truncate()
                .help_when_truncated(why),
        );
    }
    let mut actions = row().gap(8.).center_y();
    let busy = s.integrations.busy;
    let setup = |label: &str, force: bool| {
        let agents = vec![id.clone()];
        wait(
            pill(
                &format!("agent-setup:{id}"),
                label,
                Pill::Neutral,
                CONTROL,
                move |s, _| {
                    s.preview_agents(AgentChange::Setup {
                        agents: agents.clone(),
                        force,
                    })
                },
            ),
            busy,
        )
    };
    match installed {
        Installed::No if harness.detected => actions = actions.child(setup("Install…", false)),
        Installed::Outdated => actions = actions.child(setup("Update…", false)),
        Installed::Conflict => actions = actions.child(setup("Replace…", true)),
        _ => {}
    }
    if harness.has_rex_files() {
        let agents = vec![id.clone()];
        actions = actions.child(wait(
            pill(
                &format!("agent-remove:{id}"),
                "Remove…",
                Pill::Neutral,
                CONTROL,
                move |s, _| {
                    s.preview_agents(AgentChange::Remove {
                        agents: agents.clone(),
                    })
                },
            ),
            busy,
        ));
    }
    row_frame(false)
        .key(format!("agent:{id}"))
        .child(line)
        .child(actions)
}

/// `A`, `A and B`, `A, B and C`.
fn list_of(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The skill directories two or more agents on this machine read, each with the agents that
/// read it, in the order the agents are listed. A change to one of them changes the skill
/// for all.
pub fn shared_skills(harnesses: &[Harness]) -> Vec<(String, Vec<&str>)> {
    let mut shared: Vec<(String, Vec<&str>)> = Vec::new();
    let known = AGENTS
        .iter()
        .filter_map(|(id, _)| harnesses.iter().find(|h| h.id == *id));
    let others = harnesses
        .iter()
        .filter(|h| !AGENTS.iter().any(|(id, _)| *id == h.id));
    for harness in known.chain(others).filter(|h| h.detected) {
        let Some(path) = harness.skill_path() else {
            continue;
        };
        match shared.iter_mut().find(|(p, _)| p == path) {
            Some((_, agents)) => agents.push(agent_name(&harness.id)),
            None => shared.push((path.to_owned(), vec![agent_name(&harness.id)])),
        }
    }
    shared.retain(|(_, agents)| agents.len() > 1);
    shared
}

/// "Set up all detected", or nothing if every detected agent is set up. `rex` doesn't detect
/// OMP, so while OMP is pending the change names every pending agent; otherwise it names
/// none, which means every agent `rex` detects.
pub fn setup_all(harnesses: &[Harness]) -> Option<AgentChange> {
    let pending: Vec<String> = harnesses
        .iter()
        .filter(|h| h.detected && h.installed() != Installed::Yes)
        .map(|h| h.id.clone())
        .collect();
    if pending.is_empty() {
        return None;
    }
    let agents = if pending.iter().any(|id| id == "omp") {
        pending
    } else {
        Vec::new()
    };
    Some(AgentChange::Setup {
        agents,
        force: false,
    })
}

pub fn section(s: &Workspace) -> El {
    let note = footnote(
        "Installs a Rex skill and, where the agent has them, session hooks. Every change is shown first.",
    );
    let refresh = pill(
        "agents-refresh",
        "Check again",
        Pill::Neutral,
        CONTROL,
        |s, _| {
            s.integrations.status = AgentStatus::Unknown;
            s.refresh_integrations();
        },
    );
    let agents = match &s.integrations.status {
        AgentStatus::Unknown | AgentStatus::Checking => vec![
            card([row_frame(false).child(
                text("Checking agents…")
                    .key("agents-checking")
                    .font(ui(13.))
                    .color(rgb(muted())),
            )]),
            note,
        ],
        AgentStatus::Unavailable(words) => {
            vec![card([setting("Agents unavailable", words, refresh)]), note]
        }
        AgentStatus::Ready(harnesses) => {
            let known = AGENTS
                .iter()
                .filter_map(|(id, _)| harnesses.iter().find(|h| h.id == *id));
            let others = harnesses
                .iter()
                .filter(|h| !AGENTS.iter().any(|(id, _)| *id == h.id));
            // The page's actions end where its rows' controls do, the one for every agent
            // last.
            let mut all = row()
                .w_full()
                .justify_end()
                .gap(8.)
                .px(INSET)
                .child(refresh);
            if let Some(change) = setup_all(harnesses) {
                all = all.child(wait(
                    pill(
                        "agents-setup-all",
                        "Set up all detected…",
                        Pill::Primary,
                        CONTROL,
                        move |s, _| s.preview_agents(change.clone()),
                    ),
                    s.integrations.busy,
                ));
            }
            let mut blocks = vec![
                card(known.chain(others).map(|harness| agent_row(s, harness))),
                all,
                note,
            ];
            // Said once for the group, not on every row that shares.
            for (path, agents) in shared_skills(harnesses) {
                blocks.push(
                    footnote(format!(
                        "{} share one skill, so a change to one changes it for all of them:",
                        list_of(&agents)
                    ))
                    .key("agents-shared"),
                );
                blocks.push(mono_footnote(home_relative(&path)).mt(-6.));
            }
            blocks
        }
    };
    page([
        settings_kit::section("Coding agents", agents),
        settings_kit::section(
            "Agent behavior",
            [
                card([setting(
                    "Let agents name sessions",
                    "Agents name the session after the task they are working on.",
                    Switch::new(s.settings.agent_session_naming)
                        .label("Let agents name sessions")
                        .hide_label()
                        .on_toggle(|s: &mut Workspace| s.toggle_session_naming())
                        .build(kit())
                        .key("agent-naming"),
                )]),
                footnote(
                    "Applies to new agent sessions, after Rex updates the agents it has set up.",
                ),
            ],
        ),
    ])
}

fn step_line(s: &Workspace, step: &Step) -> El {
    let verb = match step.change.as_str() {
        "created" => "Create",
        "updated" => "Update",
        "removed" => "Remove",
        "unchanged" => "Keep",
        "absent" => "Already gone:",
        "conflict" => "Conflict:",
        "skipped" => "Skip",
        other => other,
    };
    let path = step.path.as_deref().map(home_relative).unwrap_or_default();
    let mut agents: Vec<&str> = step.harnesses.iter().map(|h| agent_name(h)).collect();
    if step.kind == "skill" {
        for h in &step.harnesses {
            for other in s.sharing(h) {
                if !agents.contains(&other) {
                    agents.push(other);
                }
            }
        }
    }
    let mut line = format!("{verb} {} {path}", step.kind);
    if !agents.is_empty() {
        line.push_str(&format!(" · {}", agents.join(", ")));
    }
    if let Some(reason) = &step.reason {
        line.push_str(&format!(" ({reason})"));
    }
    text(line).font(ui(12.)).color(rgb(ink())).wrap()
}

/// The preview of an agent change: what `rex` reported it would do, and Keep / Apply.
pub fn confirm(s: &Workspace, title: &str, change: &AgentChange, steps: &[Step]) -> El {
    let shown: Vec<_> = steps
        .iter()
        .filter(|st| !matches!(st.change.as_str(), "absent" | "unchanged" | "skipped"))
        .collect();
    let mut list = col().gap(6.);
    if shown.is_empty() {
        list = list.child(text("Nothing to change.").font(ui(13.)).color(rgb(muted())));
    }
    for step in &shown {
        list = list.child(step_line(s, step));
    }
    let mut note = String::new();
    if let AgentChange::Setup { force: true, .. } = change {
        note.push_str("A skill named rex that Rex didn't install will be replaced. ");
    }
    if shown
        .iter()
        .any(|st| st.kind == "skill" && !s.sharing_any(st).is_empty())
    {
        note.push_str("A shared skill changes for every agent that uses it. ");
    }
    if shown.iter().any(|st| {
        st.kind == "skill"
            && st.harnesses.iter().any(|h| h == "pi")
            && s.sharing("pi").contains(&"OMP")
    }) {
        note.push_str(match change {
            AgentChange::Remove { .. } => "OMP and Pi share one skill, so both lose it.",
            AgentChange::Setup { .. } => "OMP and Pi share one skill, so both get it.",
        });
    }
    let note = note.trim_end();
    let action = match change {
        AgentChange::Setup { force: true, .. } => "Replace",
        AgentChange::Setup { .. } => "Set up",
        AgentChange::Remove { .. } => "Remove",
    };
    let kind = match change {
        AgentChange::Remove { .. } => Pill::Danger,
        AgentChange::Setup { force: true, .. } => Pill::Warning,
        AgentChange::Setup { .. } => Pill::Primary,
    };
    let mut body = col()
        .gap(12.)
        .child(text(title.to_owned()).font(strong(15.)).color(rgb(ink())))
        .child(list);
    if !note.is_empty() {
        body = body.child(text(note).font(ui(12.)).color(rgb(muted())).wrap());
    }
    body.child(
        row()
            .gap(8.)
            .justify_end()
            .child(
                pill("agents-keep", "Keep as is", Pill::Neutral, 32., |s, tx| {
                    s.cancel_panel(tx)
                })
                .default_button()
                .tab_index(1),
            )
            .child(
                wait(
                    pill("agents-apply", action, kind, 32., |s, tx| s.submit(tx)),
                    shown.is_empty(),
                )
                .tab_index(2),
            ),
    )
}

impl Workspace {
    fn sharing_any(&self, step: &Step) -> Vec<&'static str> {
        step.harnesses
            .iter()
            .flat_map(|h| self.sharing(h))
            .collect()
    }
}
