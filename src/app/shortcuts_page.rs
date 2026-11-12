//! Settings → Shortcuts: one row for every action, bound or not, and a filter that takes a
//! name or a key.
//!
//! - [`rows`] reads the effective keymap into rows: every registered action (an action with
//!   a direction is a row a direction), then what the host's server offers. A row carries
//!   every binding that runs it, in the order keys are tried, and says in words where a
//!   binding comes from and which other action its key was also given to.
//! - [`listing`] narrows the rows to a query. `ctrl+k` is read as a key and lists what is
//!   bound to it in priority order within each mode; anything else is looked for in the names
//!   and the keys.
//! - [`filter_field`] and [`page`] draw them, from `settings_kit` and `keycaps`.
use super::{
    actions::{self, Kind},
    commands::Run,
    keycaps::{Chords, keycaps, spell},
    panel::Intent,
    recorder::{self, Recorder},
    settings_kit::{
        ChipTone, INSET, card, footnote, page as stack, row_frame, section, state_chip, words,
    },
    settings_view::SHORTCUT_CATEGORIES,
    theme::*,
    workspace::Workspace,
};
use rex_rune::keymap::{Chord, Key as BoundKey, RECORDED, Resolved};
use rexrs::api::client::{Binding, InspectResponseAction, Removal};
use rune::{El, Key, KeyEvent, Tx, col, row, text};
use rune_kit::TextInput;
use serde_json::{Value, json};

/// The filter's field and the space under it, above the page's scroll region.
pub const FILTER: f32 = 44.;
const FIELD: f32 = 32.;
/// The Settings page's scroll region while it shows Shortcuts.
const SCROLL: &str = "settings-scroll:shortcuts";
/// The heading of what the host's server offers.
const SERVER: &str = "Server";

const HERE: &str = "Recorded here";
const HELP: &str = "Press a row’s keys to replace or remove them, or Record shortcut to give it some. Filter by a name or a key, such as ctrl+k.";
const LAYERS: &str = "Shortcuts recorded here override shortcuts.json, which overrides the host's Rex configuration and the Linux defaults. Keys in a named mode apply only while that mode is active; within each mode, bindings are listed in priority order.";

/// One action on the page.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The heading it is listed under: one of `SHORTCUT_CATEGORIES`.
    pub category: &'static str,
    /// As Commands and the menus name it.
    pub title: String,
    pub action: String,
    pub args: Option<Value>,
    /// The bindings that run it, in the order keys are tried; none reads `Not set`.
    pub keys: Vec<Chords>,
    /// Its helper line, part by part: where a binding comes from, a numpad twin, an action
    /// that shares or took one of its keys.
    pub notes: Vec<String>,
    /// How the pointer runs it (`Mouse Back`), for the two actions it does.
    pub pointer: Option<&'static str>,
    /// Whether this row has valid arguments for an actual binding.
    pub recordable: bool,
    /// Every binding, including those shadowed by higher layers.
    pub held: Vec<Held>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    place: usize,
    pub chords: Vec<Chord>,
    pub binding: Binding,
    pub live: bool,
    folded: bool,
}

impl Row {
    /// A name for the row's elements: the action, then its argument's value.
    pub fn id(&self) -> String {
        let mut id = self.action.clone();
        for value in self
            .args
            .iter()
            .filter_map(Value::as_object)
            .flat_map(|args| args.values())
        {
            id.push(':');
            match value {
                Value::String(word) => id.push_str(word),
                other => id.push_str(&other.to_string()),
            }
        }
        // A key path's separator is not part of a name.
        id.replace('/', "_")
    }

    /// The bindings whose keys the row shows, in the order of `keys`.
    pub fn shown(&self) -> impl Iterator<Item = &Held> {
        self.held.iter().filter(|held| held.live && !held.folded)
    }

    /// Whether the row is for `action` with these arguments (including its condition).
    pub fn runs(&self, action: &str, args: Option<&Value>) -> bool {
        identity(&self.action, self.args.as_ref()) == identity(action, args)
    }

    /// Whether `needle` (lower case, single spaces) is in the row's name, its action, its
    /// heading or the keycaps of one of its bindings.
    fn mentions(&self, needle: &str) -> bool {
        self.title.to_lowercase().contains(needle)
            || self.action.contains(needle)
            || self.category.to_lowercase().contains(needle)
            || (self.keys.is_empty() && needle == "not set")
            || self
                .pointer
                .is_some_and(|words| words.to_lowercase().contains(needle))
            || self
                .held
                .iter()
                .any(|held| spell(&caps(&held.chords)).to_lowercase().contains(needle))
            || self
                .notes
                .iter()
                .any(|note| note.to_lowercase().contains(needle))
    }
}

fn caps(chords: &[Chord]) -> Chords {
    chords.iter().map(Chord::keycaps).collect()
}

/// One binding the keymap can read and run, at its place in the order keys are tried.
struct Bound<'a> {
    binding: &'a Binding,
    chords: &'a [Chord],
    /// The binding tried before it that has its key: the same chords, or a sequence one of
    /// them starts. Such a binding never runs.
    taken_by: Option<usize>,
}

/// The keymap's readable, runnable bindings in try order. Those nothing has taken are
/// `actions::effective`.
fn bound<'a>(map: &'a Resolved, server: &[InspectResponseAction]) -> Vec<Bound<'a>> {
    let mut list: Vec<Bound<'a>> = Vec::new();
    for candidate in &map.bindings {
        let Some(chords) = candidate.chords.as_deref() else {
            continue;
        };
        if matches!(actions::classify(&candidate.binding, server), Kind::Skip) {
            continue;
        }
        let taken_by = list.iter().position(|higher| {
            higher.taken_by.is_none()
                && actions::covers_context(higher.binding, &candidate.binding)
                && (higher.chords.starts_with(chords) || chords.starts_with(higher.chords))
        });
        list.push(Bound {
            binding: &candidate.binding,
            chords,
            taken_by,
        });
    }
    list
}

/// What tells one row from another: the action and its arguments, none and `{}` alike.
pub(super) type Identity = (String, String);

pub(super) fn identity(action: &str, args: Option<&Value>) -> Identity {
    let args = args
        .filter(|args| !args.is_null() && args.as_object().is_none_or(|a| !a.is_empty()))
        .map(Value::to_string)
        .unwrap_or_default();
    (action.to_owned(), args)
}

/// A row before its bindings are read.
struct Plan {
    category: &'static str,
    title: String,
    action: String,
    args: Option<Value>,
    /// What an action that cannot be bound as it is needs said about it.
    hint: Option<String>,
}

impl Plan {
    fn identity(&self) -> Identity {
        identity(&self.action, self.args.as_ref())
    }
}

/// Arguments in the order their rows are listed: numbers by value, then words.
fn argument_order(args: &Value) -> (i64, String) {
    let number = args
        .as_object()
        .and_then(|args| args.values().next())
        .and_then(Value::as_i64);
    (number.unwrap_or(i64::MAX), args.to_string())
}

/// Arguments as a row's title says them: `direction: left`.
fn argument_words(args: &Value) -> String {
    match args.as_object() {
        Some(args) => args
            .iter()
            .map(|(name, value)| match value {
                Value::String(word) => format!("{name}: {word}"),
                other => format!("{name}: {other}"),
            })
            .collect::<Vec<_>>()
            .join(", "),
        None => args.to_string(),
    }
}

/// Presentation metadata from the existing schema, not another action registry.
fn argument(spec: &actions::Spec) -> Option<(String, Vec<String>)> {
    let schema = spec.schema()?;
    let name = schema["required"][0].as_str()?.to_owned();
    let choices = schema["properties"][&name]["enum"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Some((name, choices))
}

/// The rows of the registered actions, in registry order: an action that runs as it is has
/// one, an action with a direction one a direction, and an action with a number, a key or
/// an id one for each value something is bound with (or one that says what it needs).
fn client_plans(groups: &[(Identity, Vec<usize>)], bound: &[Bound<'_>]) -> Vec<Plan> {
    let mut plans = Vec::new();
    let defaults = rex_rune::keymap::defaults().bindings;
    for spec in actions::SPECS {
        // Supported actions are discoverable unbound. Aliases and unsupported spellings
        // appear only when bound; diagnostics continue to explain unsupported bindings.
        let alias = super::compatibility::alias(spec.name).is_some();
        let unsupported = super::compatibility::unsupported(spec.name);
        if (alias || unsupported.is_some())
            && !groups.iter().any(|((name, _), _)| name == spec.name)
        {
            continue;
        }
        let argument = argument(spec);
        let mut listed: Vec<Option<Value>> = match &argument {
            None => vec![None],
            Some((name, choices)) => choices
                .iter()
                .map(|choice| Some(json!({ name.as_str(): choice })))
                .collect(),
        };
        if alias {
            listed.retain(|args| {
                groups
                    .iter()
                    .any(|(id, _)| *id == identity(spec.name, args.as_ref()))
            });
        }
        let mut others: Vec<&Value> = groups
            .iter()
            .filter(|((action, _), _)| action == spec.name)
            .filter(|(id, _)| {
                !listed
                    .iter()
                    .any(|args| identity(spec.name, args.as_ref()) == *id)
            })
            .filter_map(|(_, indices)| bound[indices[0]].binding.args.as_ref())
            .collect();
        // Keep argumented default rows available after their last key is removed.
        for default in defaults.iter().filter(|b| b.action == spec.name) {
            if let Some(args) = default.args.as_ref()
                && !listed
                    .iter()
                    .any(|a| identity(spec.name, a.as_ref()) == identity(spec.name, Some(args)))
                && !others.contains(&args)
            {
                others.push(args);
            }
        }
        others.sort_by_cached_key(|args| argument_order(args));
        listed.extend(others.into_iter().cloned().map(Some));
        let unbindable = listed.is_empty();
        if unbindable {
            listed.push(None);
        }
        for args in listed {
            let mut binding = Binding::new(Vec::<String>::new(), spec.name);
            binding.args = args.clone();
            let mut title = actions::binding_title(&binding);
            if title == spec.title()
                && let Some(args) = args
                    .as_ref()
                    .filter(|_| !identity(spec.name, args.as_ref()).1.is_empty())
            {
                title = format!("{title} · {}", argument_words(args));
            }
            plans.push(Plan {
                category: spec.category(),
                title,
                action: spec.name.to_owned(),
                args,
                hint: unsupported
                    .map(str::to_owned)
                    .or_else(|| alias.then(|| format!("Alias: {}", spec.name)))
                    .or_else(|| {
                        argument.as_ref().filter(|_| unbindable).map(|(name, _)| {
                            format!("Bound in shortcuts.json with an argument: {name}")
                        })
                    }),
            });
        }
    }
    plans
}

/// The rows of what the host's server offers: `offered` (an action's name and title), each
/// once or once for every argument set it is bound with, then whatever else of the server's
/// is bound.
fn server_plans(
    groups: &[(Identity, Vec<usize>)],
    bound: &[Bound<'_>],
    server: &[InspectResponseAction],
    offered: &[(String, String)],
) -> Vec<Plan> {
    let title = |name: &str| {
        offered
            .iter()
            .find(|(offer, _)| offer == name)
            .map(|(_, title)| title.clone())
            .or_else(|| {
                server
                    .iter()
                    .find(|action| action.name == name)
                    .and_then(|action| action.title.clone())
                    .filter(|title| !title.trim().is_empty())
            })
            .unwrap_or_else(|| name.to_owned())
    };
    let plan = |action: &str, args: Option<&Value>| Plan {
        category: SERVER,
        title: match args.filter(|_| !identity(action, args).1.is_empty()) {
            Some(args) => format!("{} · {}", title(action), argument_words(args)),
            None => title(action),
        },
        action: action.to_owned(),
        args: args.cloned(),
        hint: None,
    };
    let of_server = |action: &str| actions::find(action).is_none();
    let mut plans = Vec::new();
    for (name, _) in offered.iter().filter(|(name, _)| of_server(name)) {
        let mut any = false;
        for ((action, _), indices) in groups {
            if action == name {
                plans.push(plan(name, bound[indices[0]].binding.args.as_ref()));
                any = true;
            }
        }
        if !any {
            plans.push(plan(name, None));
        }
    }
    for ((action, _), indices) in groups {
        if of_server(action) && !offered.iter().any(|(name, _)| name == action) {
            plans.push(plan(action, bound[indices[0]].binding.args.as_ref()));
        }
    }
    plans
}

/// Where a binding comes from, for one that is not a Linux default.
fn source(binding: &Binding) -> Option<&str> {
    match binding.source.as_deref() {
        Some("linux") => None,
        Some("settings") => Some("shortcuts.json"),
        Some(RECORDED) => Some(HERE),
        Some(source) => Some(source),
        None => Some("Server"),
    }
}

/// Whether binding `index` is the numpad's key for a binding in `live` on the main block:
/// the same modifiers and source, on `+`, `-` or `0`.
fn numpad_twin(index: usize, live: &[usize], bound: &[Bound<'_>]) -> bool {
    let [chord] = bound[index].chords else {
        return false;
    };
    let BoundKey::Physical(code) = &chord.key else {
        return false;
    };
    let equivalent = match *code {
        "NumpadAdd" => '+',
        "NumpadSubtract" => '-',
        "Numpad0" => '0',
        _ => return false,
    };
    live.iter().any(|&other| {
        bound[other].binding.source == bound[index].binding.source
            && bound[other].binding.mode == bound[index].binding.mode
            && matches!(bound[other].chords, [twin]
                if twin.modifiers == chord.modifiers
                    && twin.key == BoundKey::Character(equivalent))
    })
}

/// Every row of the page for `map`, the keymap in effect. `server` is what the host's
/// server can run, and `offered` those of its actions that are listed even while unbound,
/// by name and title.
pub fn rows(
    map: &Resolved,
    server: &[InspectResponseAction],
    offered: &[(String, String)],
) -> Vec<Row> {
    let bound = bound(map, server);
    // The bindings of each row, rows in the order their first binding is tried.
    let mut groups: Vec<(Identity, Vec<usize>)> = Vec::new();
    for (index, found) in bound.iter().enumerate() {
        let id = identity(&found.binding.action, found.binding.args.as_ref());
        match groups.iter_mut().find(|(group, _)| *group == id) {
            Some((_, indices)) => indices.push(index),
            None => groups.push((id, vec![index])),
        }
    }
    let mut plans = client_plans(&groups, &bound);
    plans.extend(server_plans(&groups, &bound, server, offered));
    // The name of the row each binding belongs to, for another row to say.
    let mut titles = vec![String::new(); bound.len()];
    for plan in &plans {
        let id = plan.identity();
        if let Some((_, indices)) = groups.iter().find(|(group, _)| *group == id) {
            for &index in indices {
                titles[index] = plan.title.clone();
            }
        }
    }
    let keys_of = |index: usize| spell(&caps(bound[index].chords));
    plans
        .into_iter()
        .map(|plan| {
            let id = plan.identity();
            let indices: &[usize] = groups
                .iter()
                .find(|(group, _)| *group == id)
                .map(|(_, indices)| indices.as_slice())
                .unwrap_or_default();
            let live: Vec<usize> = indices
                .iter()
                .copied()
                .filter(|&index| bound[index].taken_by.is_none())
                .collect();
            let (folded, shown): (Vec<usize>, Vec<usize>) = live
                .iter()
                .partition(|&&index| numpad_twin(index, &live, &bound));
            let mut notes: Vec<String> = Vec::new();
            let mut note = |words: String| {
                if !notes.contains(&words) {
                    notes.push(words);
                }
            };
            let sources: Vec<(usize, &str)> = shown
                .iter()
                .filter_map(|&index| Some((index, source(bound[index].binding)?)))
                .collect();
            let one_source = sources.len() == shown.len()
                && sources.windows(2).all(|pair| pair[0].1 == pair[1].1);
            for (index, source) in &sources {
                note(if one_source {
                    (*source).to_owned()
                } else if *source == HERE {
                    format!("{} recorded here", keys_of(*index))
                } else {
                    format!("{} from {source}", keys_of(*index))
                });
            }
            if !folded.is_empty() {
                note("Also on the numpad".into());
            }
            for &index in indices {
                if let Some(mode) = &bound[index].binding.mode {
                    note(format!("{} in mode {mode}", keys_of(index)));
                }
                if bound[index]
                    .binding
                    .args
                    .as_ref()
                    .is_some_and(|args| args["when"] == "searching")
                {
                    note(format!("{} while the find bar is open", keys_of(index)));
                }
            }
            // A key of this row that a lower binding of another action has too.
            for (index, loser) in bound.iter().enumerate() {
                let Some(winner) = loser.taken_by else {
                    continue;
                };
                if !indices.contains(&winner)
                    || indices.contains(&index)
                    || loser.chords != bound[winner].chords
                {
                    continue;
                }
                note(if shown.len() > 1 {
                    format!("{} is also bound to {}", keys_of(winner), titles[index])
                } else {
                    format!("Also bound to {}", titles[index])
                });
            }
            // A key of this row that never reaches it.
            for &index in indices {
                if let Some(winner) = bound[index].taken_by
                    && !indices.contains(&winner)
                {
                    note(format!("{} is taken by {}", keys_of(index), titles[winner]));
                }
            }
            let recordable = actions::find(&plan.action).map_or_else(
                || {
                    server.iter().any(|a| a.name == plan.action)
                        && (plan.args.is_some()
                            || offered.iter().any(|(name, _)| *name == plan.action))
                },
                |spec| {
                    super::compatibility::unsupported(spec.name).is_none()
                        && spec.accepts(plan.args.as_ref().unwrap_or(&Value::Null))
                },
            );
            if let Some(hint) = plan.hint {
                note(hint);
            }
            let pointer = match plan.action.as_str() {
                actions::HISTORY_BACK => Some("Mouse Back"),
                actions::HISTORY_FORWARD => Some("Mouse Forward"),
                _ => None,
            };
            if let Some(pointer) = pointer.filter(|_| !shown.is_empty()) {
                note(format!("Also {pointer}"));
            }
            Row {
                category: plan.category,
                title: plan.title,
                action: plan.action,
                args: plan.args,
                keys: shown
                    .iter()
                    .map(|&index| caps(bound[index].chords))
                    .collect(),
                notes,
                pointer,
                recordable,
                held: indices
                    .iter()
                    .map(|&index| Held {
                        place: index,
                        chords: bound[index].chords.to_vec(),
                        binding: bound[index].binding.clone(),
                        live: bound[index].taken_by.is_none(),
                        folded: folded.contains(&index),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The rows a query leaves.
#[derive(Debug, Default, PartialEq)]
pub struct Listing {
    /// For a query that names a key: its keycaps in words, and the rows with a binding on
    /// it (or on a sequence it starts), the one the key performs first.
    pub key: Option<(String, Vec<Row>)>,
    /// Whether more than one of those rows is bound to exactly that key.
    pub tie: bool,
    /// The other rows the query finds, in page order: every row for no query.
    pub rows: Vec<Row>,
}

impl Listing {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.key.as_ref().is_none_or(|(_, rows)| rows.is_empty())
    }
}

/// `rows` narrowed to `query`. A query that reads as a key (`ctrl+k`, `ctrl+b>c`, with the
/// aliases shortcuts.json takes) lists what is bound to it in the order keys are tried; a
/// key without a modifier counts as one only while something is bound to it, so `tab` and
/// `d` still find names.
pub fn listing(rows: Vec<Row>, query: &str) -> Listing {
    let query = query.trim();
    if query.is_empty() {
        return Listing {
            rows,
            ..Default::default()
        };
    }
    let chords: Option<Vec<Chord>> = query
        .split('>')
        .map(|chord| Chord::parse(chord).ok())
        .collect();
    let place = |row: &Row, chords: &[Chord]| {
        row.held
            .iter()
            .filter(|held| held.chords.starts_with(chords))
            .map(|held| held.place)
            .min()
    };
    let chords = chords.filter(|chords| {
        query.contains(['+', '>']) || rows.iter().any(|row| place(row, chords).is_some())
    });
    let needle = query
        .to_lowercase()
        .replace('+', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut on_key: Vec<(usize, Row)> = Vec::new();
    let mut rest = Vec::new();
    for row in rows {
        match chords.as_deref().and_then(|chords| place(&row, chords)) {
            Some(place) => on_key.push((place, row)),
            None if chords.is_none() && row.mentions(&needle) => rest.push(row),
            None => {}
        }
    }
    on_key.sort_by_key(|(place, _)| *place);
    let tie = chords.as_ref().is_some_and(|chords| {
        let mut modes = std::collections::HashSet::new();
        on_key.iter().any(|(_, row)| {
            row.held
                .iter()
                .filter(|held| held.chords == *chords)
                .any(|held| !modes.insert(held.binding.mode.clone()))
        })
    });
    Listing {
        key: chords.map(|chords| {
            (
                spell(&caps(&chords)),
                on_key.into_iter().map(|(_, row)| row).collect(),
            )
        }),
        tie,
        rows: rest,
    }
}

/// The binding that has `chords` now, or a sequence they start, or the start of theirs: the
/// one a new binding of `chords` would take the key from. `except` is a binding about to go.
pub fn holder<'a>(
    rows: &'a [Row],
    chords: &[Chord],
    candidate: &Binding,
    except: Option<&Binding>,
) -> Option<(&'a Row, &'a Held)> {
    rows.iter()
        .flat_map(|row| row.held.iter().map(move |held| (row, held)))
        .filter(|(_, held)| {
            held.live
                && held.binding.mode == candidate.mode
                && except != Some(&held.binding)
                && (held.chords.starts_with(chords) || chords.starts_with(&held.chords))
        })
        .min_by_key(|(_, held)| held.place)
}

/// Say on a row which of its keys was taken off it on this page.
pub fn note_recorded_removals(rows: &mut [Row], changes: &[rex_rune::settings::recorded::Change]) {
    for change in changes.iter().filter(|change| change.remove) {
        let Ok(chords) = rex_rune::keymap::parse_keys(&change.keys) else {
            continue;
        };
        if let Some(row) = rows.iter_mut().find(|row| {
            row.runs(&change.action, change.args.as_ref())
                && !row
                    .held
                    .iter()
                    .any(|held| held.chords == chords && held.binding.mode == change.mode)
        }) {
            let mode = change
                .mode
                .as_ref()
                .map(|m| format!(" in mode {m}"))
                .unwrap_or_default();
            let note = format!("{} removed here{mode}", spell(&caps(&chords)));
            if !row.notes.contains(&note) {
                row.notes.push(note);
            }
        }
    }
}

/// Say on a row which of its default keys shortcuts.json took away, so an action that had a
/// key says where it went.
pub fn note_removals(rows: &mut [Row], removals: &[Removal]) {
    for default in rex_rune::keymap::defaults().bindings {
        let removed = removals.iter().any(|removal| {
            removal.mode.is_none()
                && removal.keys == default.keys
                && removal
                    .action
                    .as_ref()
                    .is_none_or(|action| *action == default.action)
        });
        let Some(chords) = rex_rune::keymap::parse_keys(&default.keys)
            .ok()
            .filter(|_| removed)
        else {
            continue;
        };
        let id = identity(&default.action, default.args.as_ref());
        if let Some(row) = rows.iter_mut().find(|row| {
            identity(&row.action, row.args.as_ref()) == id
                && !row
                    .held
                    .iter()
                    .any(|held| held.binding.mode.is_none() && held.chords == chords)
        }) {
            row.notes.push(format!(
                "{} removed in shortcuts.json",
                spell(&caps(&chords))
            ));
        }
    }
}

impl Workspace {
    /// Every row of Settings → Shortcuts for the host on screen.
    pub fn shortcut_rows(&self) -> Vec<Row> {
        let mut rows = self.keymap_rows();
        note_removals(
            &mut rows,
            &self.settings.shortcut_file.overrides.layer.removals,
        );
        note_recorded_removals(&mut rows, self.settings.recorded_shortcuts.changes());
        rows
    }

    fn keymap_rows(&self) -> Vec<Row> {
        let Some(snapshot) = self.keymap_snapshot() else {
            return rows(self.effective_keymap(), &[], &[]);
        };
        // What Commands lists under Server actions: the host's titled actions that need
        // no arguments.
        let offered: Vec<(String, String)> = self
            .commands()
            .into_iter()
            .filter_map(|command| match command.entry.action {
                Intent::Command(Run::ServerAction(name)) => Some((name, command.entry.label)),
                _ => None,
            })
            .collect();
        rows(self.effective_keymap(), &snapshot.reply.actions, &offered)
    }

    fn clear_shortcut_filter(&mut self, tx: &mut Tx) {
        self.shortcut_filter.clear();
        tx.scroll_to_y(SCROLL, 0.);
    }
}

/// Keys the filter takes: Escape empties it, and closes Settings once it is empty.
fn filter_key(s: &mut Workspace, event: &KeyEvent, tx: &mut Tx) -> bool {
    // TextInput would otherwise use Alt+arrows for caret movement before the dialog sees them.
    if event.mods.alt && s.settings_key(event, tx) || super::input::global_key(s, event, tx) {
        return true;
    }
    if event.key != Key::Escape {
        return false;
    }
    if s.shortcut_filter.text.is_empty() {
        if !s.settings_key(event, tx) {
            s.cancel_panel(tx);
        }
    } else {
        s.clear_shortcut_filter(tx);
    }
    true
}

/// The filter, which stays above the page while its rows scroll.
pub fn filter_field(s: &Workspace, inset: f32) -> El {
    let mut field = TextInput::new(&s.shortcut_filter)
        .placeholder("Filter shortcuts…")
        .label("Filter shortcuts")
        .leading_icon(&SEARCH)
        .leading_icon_size(14.)
        .text_style(ui(13.))
        .height(FIELD)
        .on_edit(|s: &mut Workspace| &mut s.shortcut_filter)
        // The results start at the top of the page, wherever it was scrolled to.
        .on_change(|_: &mut Workspace, tx: &mut Tx| tx.scroll_to_y(SCROLL, 0.))
        .on_key(filter_key)
        .width(None);
    if !s.shortcut_filter.text.is_empty() {
        field = field
            .clear_button()
            .on_action(|s: &mut Workspace, tx: &mut Tx| s.clear_shortcut_filter(tx));
    }
    row()
        .w_full()
        .h(FILTER)
        .px(inset)
        .gap(8.)
        .child(field.build(kit()).key("shortcut-filter").grow(1.).min_w(0.))
        .child(recorder::menu_button())
}

/// A row: what it runs on the left, in the name Commands and the menus use, and every
/// binding of it on the right as the same keycaps, one under another. Each binding is a
/// button that records another in its place, and a row with none has Record shortcut. The
/// row `recording` is on shows the recorder instead, and what its keys would do.
fn row_el(found: &Row, recording: Option<&(Recorder, recorder::Verdict)>) -> El {
    let id = found.id();
    if let Some((recorder, verdict)) = recording.filter(|(recorder, _)| recorder.row == id) {
        let (status, warn) = recorder.status(verdict);
        let status = text(status)
            .key("shortcut-recording-status")
            .font(ui(11.5))
            .color(rgb(if warn { warning() } else { muted() }))
            .wrap();
        // The recorder beside the name, and what it says on a line of its own: the words
        // keep their width whatever the recorder shows.
        return col()
            .key(format!("shortcut:{id}"))
            .w_full()
            .px(INSET)
            .py(6.)
            .gap(4.)
            .child(
                row()
                    .w_full()
                    .gap(12.)
                    .center_y()
                    .child(
                        text(found.title.clone())
                            .font(ui(13.))
                            .color(rgb(ink()))
                            .truncate()
                            .help_when_truncated(found.title.clone())
                            .grow(1.)
                            .shrink()
                            .min_w(96.),
                    )
                    .child(recorder::control(recorder).key(format!("shortcut-keys:{id}"))),
            )
            .child(status);
    }
    let mut notes = found.notes.clone();
    // The pointer's way to run it, which the keys' place said until it could take a key.
    if let Some(pointer) = found.pointer.filter(|_| found.keys.is_empty()) {
        notes.insert(0, pointer.to_owned());
    }
    let detail = notes.join(" · ");
    let keys = if !found.keys.is_empty() {
        let said: Vec<String> = found.keys.iter().map(|chords| spell(chords)).collect();
        col().gap(4.).items_end().label(said.join(" or ")).children(
            found.keys.iter().enumerate().map(|(index, chords)| {
                if found.recordable {
                    recorder::key_button(&id, index, &found.title, chords)
                } else {
                    keycaps(chords, false)
                }
            }),
        )
    } else if found.recordable {
        row()
            .label("Not set")
            .child(recorder::record_button(&id, &found.title))
    } else {
        // Words, not a control: the action needs an argument no key recorded here gives.
        text("Not set")
            .font(ui(12.))
            .color(rgb(muted()))
            .label("Not set")
    };
    row_frame(!detail.is_empty())
        .help(found.action.clone())
        .key(format!("shortcut:{id}"))
        .child(words(&found.title, &detail))
        .child(keys.key(format!("shortcut-keys:{id}")))
}

/// The keymap's problems and the files it is read from, under the rows.
fn problems_and_files(s: &Workspace, sections: &mut Vec<El>) {
    let diagnostics = &s.effective_keymap().diagnostics;
    if !diagnostics.is_empty() || s.model().keymap_error.is_some() {
        let problem = |words: String| {
            text(words)
                .font(ui(11.5))
                .color(rgb(warning()))
                .wrap()
                .grow(1.)
                .shrink()
        };
        let mut rows = Vec::new();
        if let Some(error) = &s.model().keymap_error {
            rows.push(row_frame(false).child(problem(error.clone())));
        }
        for (index, found) in diagnostics.iter().enumerate() {
            rows.push(
                row_frame(false).child(
                    problem(format!(
                        "{}: {}",
                        found.source.as_deref().unwrap_or("keymap"),
                        found.message
                    ))
                    .key(format!("shortcut-problem-{index}")),
                ),
            );
        }
        sections.push(section("Problems", [card(rows)]));
    }
    // Review fix from the preserved page: labelled, single-line paths, never a tall
    // paragraph of directories. The whole path remains selectable and in its tooltip.
    let file = |label: &str, key: &str, path: String, missing: Option<&str>| {
        let mut line = row_frame(false).child(
            text(label.to_owned())
                .font(ui(13.))
                .color(rgb(ink()))
                .grow(1.),
        );
        if let Some(key) = missing {
            line = line.child(state_chip(
                key,
                "Not created",
                ChipTone::Off,
                "The file does not exist yet.",
            ));
        }
        line.child(
            text(path.clone())
                .key(key.to_owned())
                .font(mono(11.))
                .color(rgb(muted()))
                .selectable(true)
                .truncate_middle()
                .help_when_truncated(path)
                .shrink()
                .min_w(0.),
        )
    };
    let mut files = Vec::new();
    if let Some(snapshot) = s.keymap_snapshot() {
        files.push(file(
            "Server configuration",
            "server-shortcuts-path",
            snapshot.reply.config.path.clone(),
            (!snapshot.reply.config.exists).then_some("server-shortcuts-missing"),
        ));
    }
    files.push(file(
        "shortcuts.json",
        "shortcuts-path",
        rex_rune::settings::shortcuts::Shortcuts::path(&s.settings_path)
            .display()
            .to_string(),
        None,
    ));
    sections.push(section(
        "Files",
        [
            card(files),
            footnote(LAYERS).key("shortcuts-layers"),
            footnote(
                "Edit shortcuts.json, then return to Rex to reload. Recorded shortcuts are saved in workspace.json, never in shortcuts.json. Escape cancels a sequence. An active key mode may block otherwise unbound keys.",
            ),
        ],
    ));
}

/// The page under the filter: every row under its heading, or what the filter leaves.
pub fn page(s: &Workspace) -> El {
    let query = s.shortcut_filter.text.trim().to_owned();
    let all = s.shortcut_rows();
    // What the keys typed into the recorder would do, said on its row.
    let recording = s
        .recording()
        .map(|recorder| (recorder.clone(), recorder.verdict(&all)));
    let row_el = |row: &Row| row_el(row, recording.as_ref());
    let found = listing(all, &query);
    let mut sections = Vec::new();
    if query.is_empty() {
        sections.push(footnote(HELP).key("shortcuts-help"));
    }
    if found.is_empty() {
        let nothing = match &found.key {
            Some((keys, _)) => format!("Nothing is bound to {keys}."),
            None => format!("No action or key matches “{query}”."),
        };
        sections.push(footnote(nothing).key("shortcuts-none"));
        return stack(sections);
    }
    if let Some((keys, rows)) = &found.key {
        let mut body = Vec::new();
        if rows.is_empty() {
            body.push(footnote("Nothing is bound to this key.").key("shortcuts-none"));
        } else {
            body.push(card(rows.iter().map(&row_el)).key("shortcuts-on-key"));
        }
        if found.tie {
            body.push(footnote(
                "Within each mode, the first binding takes priority.",
            ));
        }
        sections.push(section(&format!("Bound to {keys}"), body));
    }
    for name in SHORTCUT_CATEGORIES {
        let rows: Vec<El> = found
            .rows
            .iter()
            .filter(|row| row.category == name)
            .map(&row_el)
            .collect();
        if !rows.is_empty() {
            sections.push(section(name, [card(rows)]));
        }
    }
    if query.is_empty() {
        problems_and_files(s, &mut sections);
    }
    stack(sections)
}
