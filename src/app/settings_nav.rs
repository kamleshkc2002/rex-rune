//! Settings history and search, recovered from the preserved Wave 2 navigation.
//! Existing sections plus Advanced font settings; no placeholder pages or shortcut action search.
//! The row registry is read from settings_kit's labelled rows, not a second copy of UI text.
use super::{
    panel::{PanelKind, SettingsSection},
    theme::*,
    workspace::Workspace,
};
use rune::{El, Key, KeyEvent, SpringPair, TextEdit, Transition, Tx, el};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

pub const HEADER: f32 = 56.;
pub const PAGE_INSET: f32 = 20.;
const MARKED_FOR: Duration = Duration::from_millis(2400);
const HITS: usize = 40;
const STOPS: usize = 64;

/// One focused subpage; it participates in the same history and automatic row index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPage {
    AdvancedFont,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Section(SettingsSection),
    Sub(SubPage),
}
impl From<SettingsSection> for Page {
    fn from(section: SettingsSection) -> Self {
        Self::Section(section)
    }
}
impl PartialEq<SettingsSection> for Page {
    fn eq(&self, section: &SettingsSection) -> bool {
        *self == Self::Section(*section)
    }
}
impl Page {
    fn all() -> impl Iterator<Item = Self> {
        SettingsSection::ALL
            .into_iter()
            .map(Self::Section)
            .chain([Self::Sub(SubPage::AdvancedFont)])
    }
    pub fn section(self) -> SettingsSection {
        match self {
            Self::Section(section) => section,
            Self::Sub(SubPage::AdvancedFont) => SettingsSection::Terminal,
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Section(section) => section.title(),
            Self::Sub(SubPage::AdvancedFont) => "Advanced font settings",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Section(section) => section.key(),
            Self::Sub(SubPage::AdvancedFont) => "advanced-font",
        }
    }
}

/// Discovery aliases name existing pages, not promises of controls on them.
fn keywords(page: Page) -> &'static str {
    match page {
        Page::Sub(SubPage::AdvancedFont) => "font leading tracking spacing fallback bold italic",
        Page::Section(section) => match section {
            SettingsSection::Interface => {
                "appearance layout window scale zoom sidebar tabs downloads colors colours palette"
            }
            SettingsSection::Terminal => "font shell text padding",
            SettingsSection::Shortcuts => "keys keyboard bindings keymap hotkeys",
            SettingsSection::Integrations => "agents skill hooks claude codex opencode pi",
            SettingsSection::Notifications => "alerts sound bell desktop activity status",
            SettingsSection::Hosts => "servers ssh tailscale tailnet endpoint remote control",
            SettingsSection::Updates => "upgrade version release",
            SettingsSection::About => "version build license licence credits",
        },
    }
}

/// A page Back can return to, and how far down it was left.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stop {
    page: Page,
    scroll: f32,
}

/// A setting row, as Search Settings knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Indexed {
    page: Page,
    /// The heading of the section the row's card is under; empty for a group with no name.
    heading: String,
    label: String,
    helper: String,
}

/// One result of a search: a page, or a row on one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub page: Page,
    pub label: String,
    /// The section containing a row; empty for a page result.
    pub place: String,
    /// The hit is a row of `page`, to mark once the page shows.
    pub row: bool,
}

/// The row a search landed on, marked until `until`.
#[derive(Clone, Debug)]
struct Found {
    page: Page,
    label: String,
    until: Instant,
}

/// Where Settings is, where it has been, and its search.
#[derive(Default)]
pub struct Nav {
    pub(super) sub: Option<SubPage>,
    stops: Vec<Stop>,
    /// The stop that is on screen, once `record` has run for it.
    at: usize,
    /// How far the page on screen is scrolled, as its viewport last told.
    offset: Rc<Cell<f32>>,
    pub search: TextEdit,
    /// Every page's rows, read when a search starts and dropped when it ends.
    index: Option<Vec<Indexed>>,
    hits: Vec<Hit>,
    selected: usize,
    found: Option<Found>,
}

impl Nav {
    /// Settings just opened: no history yet and no search, on the page it was left on.
    pub fn opened(&mut self) {
        self.sub = None;
        self.stops.clear();
        self.at = 0;
        self.offset.set(0.);
        self.search = TextEdit::default();
        self.index = None;
        self.hits.clear();
        self.selected = 0;
        self.found = None;
    }

    pub fn query(&self) -> &str {
        self.search.text.trim()
    }

    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Light hit `index`, as the pointer over it does.
    pub fn select(&mut self, index: usize) {
        if index < self.hits.len() {
            self.selected = index;
        }
    }

    /// Move the lit hit down (1) or up (-1), around the ends. Returns the hit now lit.
    pub fn step_selected(&mut self, direction: isize) -> Option<usize> {
        let count = self.hits.len() as isize;
        if count == 0 {
            return None;
        }
        self.selected = (self.selected as isize + direction).rem_euclid(count) as usize;
        Some(self.selected)
    }

    /// The cell the page's viewport reports its scroll offset into.
    pub fn offset(&self) -> Rc<Cell<f32>> {
        self.offset.clone()
    }

    /// Whether `page` is the stop on screen.
    fn current(&self, page: Page) -> bool {
        self.stops
            .get(self.at)
            .is_some_and(|stop| stop.page == page)
    }

    /// Note `page`, the one on screen, and how far down it is, before leaving it. A page that
    /// was shown without a visit (Settings opening on a section) becomes a stop here, after
    /// the one Settings was on, and ends what Forward could reach.
    fn record(&mut self, page: Page) {
        let scroll = self.offset.get();
        if self.current(page) {
            self.stops[self.at].scroll = scroll;
            return;
        }
        if !self.stops.is_empty() {
            self.stops.truncate(self.at + 1);
        }
        self.stops.push(Stop { page, scroll });
        self.at = self.stops.len() - 1;
    }

    /// A visit to `page` from the recorded stop: what was ahead is forgotten.
    fn push(&mut self, page: Page) {
        self.stops.truncate(self.at + 1);
        self.stops.push(Stop { page, scroll: 0. });
        if self.stops.len() > STOPS {
            self.stops.remove(0);
        }
        self.at = self.stops.len() - 1;
        self.offset.set(0.);
    }

    /// Whether Back (-1) or Forward (1) has somewhere to go from `page`, the one on screen.
    fn can_go(&self, page: Page, direction: isize) -> bool {
        if !self.current(page) {
            // Shown without a visit: it would follow the stop Settings was on.
            return direction < 0 && !self.stops.is_empty();
        }
        self.at
            .checked_add_signed(direction)
            .is_some_and(|next| next < self.stops.len())
    }

    fn step(&mut self, direction: isize) -> Option<Stop> {
        let next = self.at.checked_add_signed(direction)?;
        let stop = *self.stops.get(next)?;
        self.at = next;
        self.offset.set(stop.scroll);
        Some(stop)
    }
}

thread_local! {
    /// What the page being built is built for.
    static PASS: RefCell<Pass> = const { RefCell::new(Pass::Draw(None)) };
}

enum Pass {
    /// To be drawn, with the label of the row to mark and whether it has been.
    Draw(Option<(String, bool)>),
    /// To be read for the search index: its rows so far, and where the rows under the next
    /// heading start.
    Read(Vec<(String, String, String)>, usize),
}

/// Whether the next labelled row will be marked. Its helper needs primary ink on the tint.
pub fn will_mark(label: &str) -> bool {
    PASS.with_borrow(|pass| matches!(pass, Pass::Draw(Some((found, false))) if found == label))
}

/// Every labelled row of a Settings page passes through here as it is built (`setting` and
/// `disclosure` in `settings_kit`): it is noted for the search index, or marked when it is
/// the row a search landed on.
pub fn row(label: &str, helper: &str, frame: El) -> El {
    PASS.with_borrow_mut(|pass| match pass {
        Pass::Read(rows, _) => {
            if !label.is_empty() {
                rows.push((String::new(), label.to_owned(), helper.to_owned()));
            }
            frame
        }
        Pass::Draw(Some((found, marked))) if !*marked && found.as_str() == label => {
            *marked = true;
            frame.child(mark())
        }
        Pass::Draw(_) => frame,
    })
}

/// A section's heading, told once its rows are built: the rows since the last one are under it.
pub fn heading(title: &str) {
    PASS.with_borrow_mut(|pass| {
        if let Pass::Read(rows, open) = pass {
            for row in &mut rows[*open..] {
                row.0 = title.to_owned();
            }
            *open = rows.len();
        }
    });
}

/// The mark on the row a search landed on: `SELECTION`, the tint and its outline, inside the
/// row's edges. It fades in and out and takes no input.
fn mark() -> El {
    let (fill, outline) = selection(false);
    el().key("settings-found")
        .pin(Some(3.), Some(3.), Some(3.), Some(3.))
        .radius(CARD_RADIUS - 3.)
        .continuous_corners()
        .fill(fill)
        .border(1., outline)
        .inert()
        .transition(Transition::opacity().springs(SpringPair::new(SNAPPY, POP)))
}

/// `page`'s rows, by building it.
fn read(s: &Workspace, page: Page) -> Vec<Indexed> {
    // Shortcuts owns its own filter; this search links to the page, never bypasses it.
    if page == Page::Section(SettingsSection::Shortcuts) {
        return Vec::new();
    }
    PASS.set(Pass::Read(Vec::new(), 0));
    drop(super::settings_view::body(s, page));
    let Pass::Read(rows, _) = PASS.replace(Pass::Draw(None)) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    rows.into_iter()
        // A label a page repeats (one per host) is found once, at its first row.
        .filter(|(_, label, _)| seen.insert(label.clone()))
        .map(|(heading, label, helper)| Indexed {
            page,
            heading,
            label,
            helper,
        })
        .collect()
}

/// `page`, built to be drawn, with the row a search landed on marked.
pub fn draw(s: &Workspace, page: Page) -> El {
    let found = s
        .settings_nav
        .found
        .as_ref()
        .filter(|found| found.page == page)
        .map(|found| (found.label.clone(), false));
    PASS.set(Pass::Draw(found));
    let body = super::settings_view::body(s, page);
    PASS.set(Pass::Draw(None));
    body
}

/// How well `query` (lower case; `words` are its parts) finds something named `label` that
/// also carries `more`: lower is better, None is no match.
fn rank(query: &str, words: &[&str], label: &str, more: &str) -> Option<u8> {
    let label = label.to_lowercase();
    let named: Vec<&str> = label
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    // The label begins with the query, or holds its words whole and in order: `font` finds
    // `Terminal font` as well as it finds `Font size`.
    if label.starts_with(query)
        || named
            .windows(words.len())
            .any(|run| run.iter().zip(words).all(|(word, typed)| word == typed))
    {
        return Some(0);
    }
    if named.iter().any(|word| word.starts_with(query)) {
        return Some(1);
    }
    if label.contains(query) {
        return Some(2);
    }
    if words.iter().all(|word| label.contains(word)) {
        return Some(3);
    }
    let more = more.to_lowercase();
    words
        .iter()
        .all(|word| label.contains(word) || more.contains(word))
        .then_some(4)
}

/// Pages by title and keywords, rows by label, helper and section. Best match first;
/// equal ranks keep the sidebar order and each page's row order.
fn search(index: &[Indexed], query: &str) -> Vec<Hit> {
    let query = query.trim().to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for page in Page::all() {
        if let Some(rank) = rank(&query, &words, page.title(), keywords(page)) {
            hits.push((
                rank,
                Hit {
                    page,
                    label: page.title().into(),
                    place: String::new(),
                    row: false,
                },
            ));
        }
    }
    for found in index {
        // The disclosure and its destination are one search result.
        if found.page == SettingsSection::Terminal
            && found.label == Page::Sub(SubPage::AdvancedFont).title()
        {
            continue;
        }
        let more = format!("{} {} {}", found.helper, found.heading, found.page.title());
        if let Some(rank) = rank(&query, &words, &found.label, &more) {
            hits.push((
                rank,
                Hit {
                    page: found.page,
                    label: found.label.clone(),
                    place: found.page.title().into(),
                    row: true,
                },
            ));
        }
    }
    hits.sort_by_key(|(rank, _)| *rank);
    hits.into_iter().map(|(_, hit)| hit).take(HITS).collect()
}

impl Workspace {
    fn settings_open(&self) -> bool {
        self.panel
            .as_ref()
            .is_some_and(|p| matches!(p.kind, PanelKind::Settings))
    }

    /// The page currently shown beside the sidebar.
    pub fn settings_page(&self) -> Page {
        self.settings_nav
            .sub
            .map_or(Page::Section(self.settings_section), Page::Sub)
    }

    /// Go to `page` as the user does from the sidebar, a row or a search: a new stop in the
    /// history, opened at its top.
    pub fn navigate(&mut self, page: impl Into<Page>) {
        let page = page.into();
        let from = self.settings_page();
        if page == from {
            return;
        }
        self.settings_nav.record(from);
        self.settings_nav.push(page);
        self.settings_nav.found = None;
        self.show_settings_page(page);
    }

    fn show_settings_page(&mut self, page: Page) {
        self.open_section(page.section());
        self.settings_nav.sub = match page {
            Page::Section(_) => None,
            Page::Sub(sub) => Some(sub),
        };
    }

    pub fn open_sub_page(&mut self, sub: SubPage, tx: &mut Tx) {
        self.navigate(Page::Sub(sub));
        tx.scroll_to_y(
            format!("settings-scroll:{}", self.settings_page().key()),
            0.,
        );
        tx.focus("settings-back", tx.focus_visible());
    }

    /// Whether Back (-1) or Forward (1) has a page to show.
    pub fn settings_can_go(&self, direction: isize) -> bool {
        self.settings_nav.can_go(self.settings_page(), direction)
    }

    /// Back (-1) or Forward (1) through the pages visited, each where it was scrolled.
    /// Returns whether Settings is what is open, and so whether the step was Settings' to take.
    pub fn settings_history(&mut self, direction: isize, tx: &mut Tx) -> bool {
        if !self.settings_open() {
            return false;
        }
        let from = self.settings_page();
        if !self.settings_nav.can_go(from, direction) {
            return true;
        }
        self.settings_nav.record(from);
        if let Some(stop) = self.settings_nav.step(direction) {
            self.settings_nav.found = None;
            self.show_settings_page(stop.page);
            tx.scroll_to_y(format!("settings-scroll:{}", stop.page.key()), stop.scroll);
            if matches!(from, Page::Sub(SubPage::AdvancedFont))
                && stop.page == SettingsSection::Terminal
                && !self.settings_nav.search.focused
            {
                tx.focus("open-advanced-font", tx.focus_visible());
            }
        }
        true
    }

    /// The search field's text changed: find again, from the first hit.
    pub fn settings_search_changed(&mut self) {
        if self.settings_nav.query().is_empty() {
            self.settings_nav.index = None;
            self.settings_nav.hits.clear();
            self.settings_nav.selected = 0;
            return;
        }
        if self.settings_nav.index.is_none() {
            self.settings_nav.index = Some(Page::all().flat_map(|page| read(self, page)).collect());
        }
        let nav = &mut self.settings_nav;
        nav.hits = search(
            nav.index.as_deref().unwrap_or_default(),
            nav.search.text.trim(),
        );
        nav.selected = 0;
    }

    /// End the search: the sidebar lists the sections again.
    pub fn clear_settings_search(&mut self) {
        let focused = self.settings_nav.search.focused;
        self.settings_nav.search = TextEdit::default();
        self.settings_nav.search.focused = focused;
        self.settings_search_changed();
    }

    /// Open hit `index` of the search: its page, and for a row, the row in view and marked.
    pub fn open_settings_hit(&mut self, index: usize, tx: &mut Tx) {
        let Some(hit) = self.settings_nav.hits.get(index).cloned() else {
            return;
        };
        self.settings_nav.selected = index;
        self.navigate(hit.page);
        self.settings_nav.found = hit.row.then(|| Found {
            page: hit.page,
            label: hit.label,
            until: Instant::now() + MARKED_FOR,
        });
        if hit.row {
            tx.scroll_center("settings-found");
            tx.wake_after(MARKED_FOR);
        }
    }

    /// Take the mark off the row a search landed on once its time is up.
    pub fn poll_settings(&mut self, now: Instant, tx: &mut Tx) -> bool {
        let Some(found) = &self.settings_nav.found else {
            return false;
        };
        if now < found.until && self.settings_open() {
            tx.wake_after(found.until - now);
            return false;
        }
        self.settings_nav.found = None;
        true
    }

    /// Reserved Settings keys stay local, whichever control has focus.
    pub fn settings_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool {
        if ev.key == Key::Escape
            && self.settings_open()
            && !self.settings_nav.search.text.is_empty()
        {
            self.clear_settings_search();
            return true;
        }
        if ev.key == Key::Escape && self.settings_open() && self.settings_nav.sub.is_some() {
            let parent = Page::Section(SettingsSection::Terminal);
            if self
                .settings_nav
                .at
                .checked_sub(1)
                .and_then(|at| self.settings_nav.stops.get(at))
                .is_some_and(|stop| stop.page == parent)
            {
                self.settings_history(-1, tx);
            } else {
                self.navigate(parent);
                tx.scroll_into_view("open-advanced-font");
            }
            tx.focus("open-advanced-font", tx.focus_visible());
            return true;
        }
        if !ev.mods.alt || ev.mods.ctrl || ev.mods.cmd || ev.mods.shift {
            return false;
        }
        let direction = match ev.key {
            Key::Left => -1,
            Key::Right => 1,
            _ => return false,
        };
        self.settings_history(direction, tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERFACE: Page = Page::Section(SettingsSection::Interface);
    const TERMINAL: Page = Page::Section(SettingsSection::Terminal);
    const ABOUT: Page = Page::Section(SettingsSection::About);
    const PANES: Page = Page::Section(SettingsSection::Notifications);

    fn visit(nav: &mut Nav, from: Page, to: Page, scrolled: f32) {
        nav.offset.set(scrolled);
        nav.record(from);
        nav.push(to);
    }

    #[test]
    fn history_branches_and_keeps_each_stops_scroll() {
        let mut nav = Nav::default();
        assert!(!nav.can_go(INTERFACE, -1) && !nav.can_go(INTERFACE, 1));
        visit(&mut nav, INTERFACE, PANES, 120.);
        visit(&mut nav, PANES, TERMINAL, 40.);
        assert!(nav.can_go(TERMINAL, -1) && !nav.can_go(TERMINAL, 1));

        nav.offset.set(300.);
        nav.record(TERMINAL);
        assert_eq!(
            nav.step(-1),
            Some(Stop {
                page: PANES,
                scroll: 40.
            })
        );
        assert_eq!(nav.offset.get(), 40., "the page comes back where it was");
        nav.record(PANES);
        assert_eq!(
            nav.step(-1).map(|s| (s.page, s.scroll)),
            Some((INTERFACE, 120.))
        );
        assert!(nav.step(-1).is_none());
        assert!(nav.can_go(INTERFACE, 1));
        nav.record(INTERFACE);
        assert_eq!(nav.step(1).map(|s| s.page), Some(PANES));
        nav.record(PANES);
        assert_eq!(
            nav.step(1).map(|s| (s.page, s.scroll)),
            Some((TERMINAL, 300.))
        );

        // A visit from the middle forgets what was ahead.
        nav.record(TERMINAL);
        nav.step(-1);
        visit(&mut nav, PANES, ABOUT, 0.);
        assert!(!nav.can_go(ABOUT, 1));
        assert_eq!(
            nav.stops.iter().map(|s| s.page).collect::<Vec<_>>(),
            [INTERFACE, PANES, ABOUT]
        );
    }

    #[test]
    fn a_page_shown_without_a_visit_becomes_a_stop_when_it_is_left() {
        let mut nav = Nav::default();
        // Settings opened on About: nothing to go back to.
        assert!(!nav.can_go(ABOUT, -1));
        visit(&mut nav, ABOUT, TERMINAL, 0.);
        assert_eq!(
            nav.stops.iter().map(|s| s.page).collect::<Vec<_>>(),
            [ABOUT, TERMINAL]
        );
        // Shown again without a visit while there is history: Back returns to where it was.
        assert!(nav.can_go(INTERFACE, -1) && !nav.can_go(INTERFACE, 1));
        nav.record(INTERFACE);
        assert_eq!(nav.step(-1).map(|s| s.page), Some(TERMINAL));

        nav.opened();
        assert!(nav.stops.is_empty() && !nav.can_go(TERMINAL, -1));
    }

    #[test]
    fn history_is_bounded() {
        let mut nav = Nav::default();
        for i in 0..200 {
            let (from, to) = if i % 2 == 0 {
                (INTERFACE, TERMINAL)
            } else {
                (TERMINAL, INTERFACE)
            };
            visit(&mut nav, from, to, 0.);
        }
        assert_eq!(nav.stops.len(), STOPS);
        assert_eq!(nav.at, STOPS - 1);
    }

    #[test]
    fn search_ranks_labels_before_helpers_and_matches_every_word() {
        let index = [
            Indexed {
                page: TERMINAL,
                heading: "Typography".into(),
                label: "Font size".into(),
                helper: "From 8 to 48 pt.".into(),
            },
            Indexed {
                page: TERMINAL,
                heading: "Spacing".into(),
                label: "Pane padding".into(),
                helper: "Space around each terminal.".into(),
            },
            Indexed {
                page: INTERFACE,
                heading: "Appearance".into(),
                label: "Theme".into(),
                helper: "Matching terminal colours.".into(),
            },
        ];
        assert_eq!(search(&index, "  PADDING ")[0].label, "Pane padding");
        assert_eq!(
            search(&index, "sound")[0].page,
            Page::Section(SettingsSection::Notifications)
        );
        assert!(!search(&index, "sound")[0].row);
        assert_eq!(search(&index, "typography")[0].label, "Font size");
        assert!(
            search(&index, "terminal padding")
                .iter()
                .any(|hit| hit.label == "Pane padding")
        );
        assert!(search(&index, "font zebra").is_empty());
        assert!(search(&index, "   ").is_empty());
    }
}
