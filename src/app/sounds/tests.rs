use super::*;
use player::tests::FakePlayer;

/// Headless Workspace always uses this fake, even if a real player is installed.
#[derive(Default)]
pub struct Fake {
    gate: Gate,
    pub player: FakePlayer,
    pub clock: Duration,
    health: Health,
}
impl Fake {
    pub fn request(&mut self, request: Request) -> Health {
        self.gate.offer(request, self.clock);
        self.flush();
        self.health
    }
    pub fn advance(&mut self, amount: Duration) {
        self.clock += amount;
        self.flush();
    }
    fn flush(&mut self) {
        let sound = self.gate.take(self.clock);
        let check = std::mem::take(&mut self.gate.check);
        if sound.is_some() || check {
            let health = player::attempt(&mut self.player, sound, || false);
            if sound.is_some()
                || health != Health::Available
                || !matches!(self.health, Health::Failed | Health::TimedOut)
            {
                self.health = health;
            }
            self.gate.busy = false;
        }
    }
}

#[test]
fn burst_chooses_most_pressing_and_drops_cooldown_events_forever() {
    let mut out = Fake::default();
    for sound in [
        Sound::Done,
        Sound::Error,
        Sound::NeedsYou,
        Sound::Done,
        Sound::Error,
    ] {
        out.request(Request::Event(sound));
    }
    assert!(out.player.lookups.is_empty());
    out.advance(GATHER);
    assert_eq!(out.player.lookups, [Sound::NeedsYou]);
    assert_eq!(out.player.invocations.len(), 1);
    out.request(Request::Event(Sound::Error));
    out.advance(INTERVAL);
    assert_eq!(out.player.invocations.len(), 1, "nothing queued for later");
    out.request(Request::Event(Sound::Done));
    out.advance(GATHER);
    assert_eq!(out.player.lookups, [Sound::NeedsYou, Sound::Done]);
}

#[test]
fn busy_drops_events_and_cancel_discards_unplayed_news() {
    let mut gate = Gate::default();
    gate.offer(Request::Event(Sound::Done), Duration::ZERO);
    assert_eq!(gate.take(GATHER), Some(Sound::Done));
    gate.offer(Request::Event(Sound::NeedsYou), INTERVAL * 4);
    gate.offer(Request::Test, INTERVAL * 4);
    gate.busy = false;
    assert_eq!(gate.take(INTERVAL * 5), None);
    gate.offer(Request::Event(Sound::Error), INTERVAL * 5);
    gate.offer(Request::Cancel, INTERVAL * 5);
    assert_eq!(gate.take(INTERVAL * 6), None);
}

#[test]
fn explicit_play_is_done_even_beside_activity_or_turning_the_switch_off() {
    let mut gate = Gate::default();
    gate.offer(Request::Test, Duration::ZERO);
    gate.offer(Request::Event(Sound::NeedsYou), Duration::ZERO);
    gate.offer(Request::Cancel, Duration::ZERO);
    assert_eq!(gate.take(Duration::ZERO), Some(Sound::Done));
}

#[test]
fn a_delayed_check_cannot_replay_stale_pending_news() {
    let mut gate = Gate::default();
    gate.offer(Request::Event(Sound::Done), Duration::ZERO);
    assert_eq!(gate.take(INTERVAL * 3), None);
    assert!(gate.pending.is_none());
}

#[test]
fn page_check_preserves_failure_and_a_later_test_can_recover() {
    let mut out = Fake::default();
    out.player.result = Some(false);
    assert_eq!(out.request(Request::Test), Health::Failed);
    assert_eq!(out.request(Request::Check), Health::Failed);
    assert_eq!(out.player.invocations.len(), 1);
    out.player.result = Some(true);
    assert_eq!(out.request(Request::Test), Health::Available);
    assert_eq!(out.player.invocations.len(), 2);
}
