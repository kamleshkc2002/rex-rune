//! Decisions shared by the D-Bus worker and the headless outbox. Failed commands are
//! consumed once; only a later command can establish another connection.
use super::{Kind, Note, NoteTarget};

/// Notifications remembered for replacing, closing and clicking. Past this the oldest is
/// forgotten (left on the desktop, no longer ours to close), so a daemon that never says
/// a notification closed cannot grow the list.
pub(super) const REMEMBERED: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Send(Note),
    Seen(NoteTarget),
    Check,
    Test,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Service {
    #[default]
    Unchecked,
    Available,
    NotFound,
    Unreachable(String),
    /// The service answered and refused the request: it is there, and reachable.
    Rejected,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum TestResult {
    #[default]
    None,
    Sending,
    Sent,
    Failed(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Health {
    pub service: Service,
    pub test: TestResult,
}

impl Service {
    pub fn reason(&self) -> &str {
        match self {
            Self::NotFound => "No notification service",
            Self::Unreachable(reason) => reason,
            Self::Rejected => "Service rejected the request",
            _ => "",
        }
    }
}

pub(super) trait Backend {
    /// A successful answer, plus the bus/daemon identity that owns notification ids.
    async fn ready(&mut self) -> Result<String, Service>;
    async fn send(&mut self, note: &Note, replaces: u32) -> Result<u32, Service>;
    async fn close(&mut self, id: u32) -> Result<(), Service>;
}

#[derive(Default)]
pub struct Ledger {
    owner: Option<String>,
    pub health: Health,
    /// Oldest first.
    live: Vec<(u32, Note)>,
    /// Notifications whose click was already followed.
    clicked: Vec<u32>,
}

impl Ledger {
    pub fn closed(&mut self, id: u32) {
        self.live.retain(|(live, _)| *live != id);
        self.clicked.retain(|clicked| *clicked != id);
    }

    /// Where a clicked notification leads: once, however often its action is reported.
    /// The id stays remembered until seeing the tab closes it.
    pub fn click(&mut self, id: u32) -> Option<NoteTarget> {
        let target = self.target(id)?;
        if self.clicked.contains(&id) {
            return None;
        }
        self.clicked.push(id);
        Some(target)
    }

    pub fn target(&self, id: u32) -> Option<NoteTarget> {
        let (_, note) = self.live.iter().find(|(live, _)| *live == id)?;
        note.target.clone()
    }

    #[cfg(test)]
    pub fn remembered(&self) -> usize {
        self.live.len()
    }

    fn replaces(&self, note: &Note) -> u32 {
        if !matches!(note.kind, Kind::Activity(_)) || note.target.is_none() {
            return 0;
        }
        self.live
            .iter()
            .find_map(|(id, old)| {
                (matches!(old.kind, Kind::Activity(_)) && old.target == note.target).then_some(*id)
            })
            .unwrap_or(0)
    }

    pub(super) async fn execute(&mut self, command: Command, bus: &mut impl Backend) {
        if let Command::Seen(target) = &command
            && !self
                .live
                .iter()
                .any(|(_, note)| note.target.as_ref() == Some(target))
        {
            return;
        }
        let test = matches!(command, Command::Test);
        let result = self.apply(command, bus).await;
        if let Err(error) = &result {
            self.health.service = error.clone();
        }
        if test {
            self.health.test = match result {
                Ok(()) => TestResult::Sent,
                Err(error) => TestResult::Failed(error.reason().into()),
            };
        }
    }

    async fn apply(&mut self, command: Command, bus: &mut impl Backend) -> Result<(), Service> {
        let owner = bus.ready().await?;
        if self.owner.as_ref() != Some(&owner) {
            self.live.clear();
            self.clicked.clear();
            self.owner = Some(owner);
        }
        self.health.service = Service::Available;
        match command {
            Command::Send(note) => self.send(note, bus).await?,
            Command::Test => self.send(Note::test(), bus).await?,
            Command::Check => {}
            Command::Seen(target) => {
                let ids: Vec<_> = self
                    .live
                    .iter()
                    .filter_map(|(id, note)| (note.target.as_ref() == Some(&target)).then_some(*id))
                    .collect();
                // Remove before calling: seeing the tab again never replays failed closes.
                self.live.retain(|(id, _)| !ids.contains(id));
                self.clicked.retain(|id| !ids.contains(id));
                for id in ids {
                    // A refusal means the daemon no longer has it, which is what was wanted.
                    // Anything else ends the attempt: a daemon that does not answer is
                    // not asked again for each remaining notification.
                    if let Err(error) = bus.close(id).await
                        && error != Service::Rejected
                    {
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }

    async fn send(&mut self, note: Note, bus: &mut impl Backend) -> Result<(), Service> {
        let replaces = self.replaces(&note);
        let id = bus.send(&note, replaces).await?;
        self.live
            .retain(|(live, _)| *live != replaces && *live != id);
        // A replacement can reuse a clicked notification's id; it is a new notification.
        self.clicked
            .retain(|clicked| *clicked != replaces && *clicked != id);
        // Test notifications have no navigation or tab lifecycle.
        if note.target.is_some() {
            self.live.push((id, note));
            if self.live.len() > REMEMBERED {
                let (forgotten, _) = self.live.remove(0);
                self.clicked.retain(|clicked| *clicked != forgotten);
            }
        }
        Ok(())
    }
}

/// Headless UI's existing notes plus the actual worker's replacement/close decisions.
#[cfg(test)]
#[derive(Default)]
pub struct Outbox {
    pub ledger: Ledger,
    pub commands: Vec<Command>,
    pub sent: Vec<(Note, u32, u32)>,
    pub closed: Vec<u32>,
    pub failure: Option<Service>,
    pub send_failure: Option<Service>,
    pub close_failure: Option<Service>,
    pub attempts: Vec<Note>,
    pub owner: Option<String>,
    pub(super) next_id: u32,
}

#[cfg(test)]
impl Backend for Outbox {
    async fn ready(&mut self) -> Result<String, Service> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        Ok(self
            .owner
            .clone()
            .unwrap_or_else(|| "private-test-daemon".into()))
    }
    async fn send(&mut self, note: &Note, replaces: u32) -> Result<u32, Service> {
        self.attempts.push(note.clone());
        if let Some(error) = self.send_failure.take() {
            return Err(error);
        }
        let id = if replaces == 0 {
            self.next_id += 1;
            self.next_id
        } else {
            replaces
        };
        self.sent.push((note.clone(), replaces, id));
        Ok(id)
    }
    async fn close(&mut self, id: u32) -> Result<(), Service> {
        self.closed.push(id);
        self.close_failure.take().map_or(Ok(()), Err)
    }
}

#[cfg(test)]
impl Outbox {
    pub fn execute(&mut self, command: Command) -> Health {
        self.commands.push(command.clone());
        let mut ledger = std::mem::take(&mut self.ledger);
        // These fake backend futures complete immediately; no desktop or runtime is used.
        use futures_util::FutureExt;
        ledger
            .execute(command, self)
            .now_or_never()
            .expect("synchronous outbox");
        let health = ledger.health.clone();
        self.ledger = ledger;
        health
    }
}
