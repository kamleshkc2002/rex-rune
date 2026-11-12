//! Shortcuts recorded in Settings → Shortcuts: a keymap layer above the user's
//! `shortcuts.json`, kept in the app's own settings file. The user's file is never written.
use crate::keymap::{Chord, Overrides, RECORDED, parse_keys};
use rexrs::api::client::{Binding, KeymapDefault, Removal};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

/// One recorded change: keys given to an action, or keys taken off it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Change {
    /// The chords in order, in Rex's spelling: `["ctrl+b", "c"]`.
    pub keys: Vec<String>,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Take the keys off the action in the layers below, instead of binding them.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remove: bool,
}

/// The recorded layer, in the order its changes were made.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Recorded(Vec<Change>);

/// A file with an entry this version cannot read keeps the rest: one bad entry is no reason
/// to refuse the whole workspace.
impl<'de> Deserialize<'de> for Recorded {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = match Value::deserialize(deserializer)? {
            Value::Array(entries) => entries,
            _ => Vec::new(),
        };
        Ok(Self(
            entries
                .into_iter()
                .filter_map(|entry| serde_json::from_value::<Change>(entry).ok())
                .filter(|change| {
                    !change.action.trim().is_empty()
                        && !change.keys.is_empty()
                        && change.keys.iter().all(|key| !key.trim().is_empty())
                })
                .collect(),
        ))
    }
}

/// No arguments, `null` and `{}` are the same thing to an action.
fn same_args(a: Option<&Value>, b: Option<&Value>) -> bool {
    let empty = |args: Option<&Value>| {
        args.is_none_or(|args| args.is_null() || args.as_object().is_some_and(|a| a.is_empty()))
    };
    (empty(a) && empty(b)) || a == b
}

/// Whether two key sequences are the same keys, however each is spelled (`ctrl+shift+d`,
/// `Shift+Control+D`). A sequence this client cannot read is only itself.
fn same_keys(a: &[String], b: &[String]) -> bool {
    let spelled = |keys: &[String]| -> Option<Vec<String>> {
        Some(parse_keys(keys).ok()?.iter().map(Chord::spelling).collect())
    };
    a == b || spelled(a).is_some_and(|a| Some(a) == spelled(b))
}

impl Recorded {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn changes(&self) -> &[Change] {
        &self.0
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Give `keys` to `action`. A key recorded for something else before goes to this.
    pub fn bind(
        &mut self,
        keys: Vec<String>,
        action: &str,
        args: Option<Value>,
        mode: Option<String>,
    ) {
        self.0.retain(|change| {
            change.remove
                || change.mode != mode
                || !same_keys(&change.keys, &keys)
                || change.args.as_ref().and_then(|a| a.get("when"))
                    != args.as_ref().and_then(|a| a.get("when"))
        });
        self.0.push(Change {
            keys,
            action: action.to_owned(),
            args,
            mode,
            remove: false,
        });
    }

    /// Forget a recorded binding. False when this layer has no such binding: it comes from
    /// a layer below, which only [`Recorded::remove`] can take it off.
    pub fn unbind(
        &mut self,
        keys: &[String],
        action: &str,
        args: Option<&Value>,
        mode: Option<&str>,
    ) -> bool {
        let before = self.0.len();
        self.0.retain(|change| {
            change.remove
                || change.mode.as_deref() != mode
                || !same_keys(&change.keys, keys)
                || change.action != action
                || !same_args(change.args.as_ref(), args)
        });
        before != self.0.len()
    }

    /// Take `keys` off `action` in the layers below. `args` only says which of the action's
    /// rows the key was on.
    pub fn remove(
        &mut self,
        keys: Vec<String>,
        action: &str,
        args: Option<Value>,
        mode: Option<String>,
    ) {
        if self.removes(&keys, action, mode.as_deref()) {
            return;
        }
        self.0.push(Change {
            keys,
            action: action.to_owned(),
            args,
            mode,
            remove: true,
        });
    }

    fn removes(&self, keys: &[String], action: &str, mode: Option<&str>) -> bool {
        self.0.iter().any(|change| {
            change.remove
                && change.mode.as_deref() == mode
                && same_keys(&change.keys, keys)
                && change.action == action
        })
    }

    /// Stop taking `keys` off `action`, so the binding below is back. False when it was
    /// never taken off.
    pub fn restore(&mut self, keys: &[String], action: &str, mode: Option<&str>) -> bool {
        let before = self.0.len();
        self.0.retain(|change| {
            !(change.remove
                && change.mode.as_deref() == mode
                && same_keys(&change.keys, keys)
                && change.action == action)
        });
        before != self.0.len()
    }

    /// The layer as Rex takes it in `client.keymap`'s overrides, after the user's file.
    pub fn layer(&self) -> KeymapDefault {
        let mut layer = KeymapDefault::new(RECORDED);
        for change in &self.0 {
            if change.remove {
                let mut removal = Removal::new(change.keys.clone()).action(change.action.clone());
                removal.mode = change.mode.clone();
                layer.removals.push(removal);
            } else {
                let mut binding = Binding::new(change.keys.clone(), change.action.clone());
                binding.args = change.args.clone();
                binding.mode = change.mode.clone();
                layer.bindings.push(binding);
            }
        }
        layer
    }

    /// The layer for local resolution. Its entries are written by the app, so there is
    /// nothing to diagnose before resolving.
    pub fn overrides(&self) -> Overrides {
        Overrides {
            layer: self.layer(),
            diagnostics: Vec::new(),
        }
    }

    /// Export the app-owned layer losslessly, including modes and action-scoped removals.
    /// This is a workspace.json fragment, not the less expressive shortcuts.json format.
    pub fn as_file(&self) -> Value {
        json!({ "recorded_shortcuts": self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keymap::{self, defaults},
        settings::Settings,
    };

    fn keys(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|key| (*key).to_owned()).collect()
    }

    /// What `keys` runs with `recorded` over the Linux defaults.
    fn runs(recorded: &Recorded, chord: &str) -> Option<String> {
        let map = keymap::resolve_all(&[defaults()], &[&recorded.overrides()], &[]);
        map.bindings
            .iter()
            .find(|bound| bound.binding.keys == keys(&[chord]))
            .map(|bound| bound.binding.action.clone())
    }

    #[test]
    fn a_recorded_key_wins_and_a_removed_one_is_gone_until_it_is_restored() {
        let mut recorded = Recorded::default();
        assert_eq!(
            runs(&recorded, "ctrl+shift+d").as_deref(),
            Some("pane.split")
        );
        recorded.remove(
            keys(&["ctrl+shift+d"]),
            "pane.split",
            Some(json!({"direction": "right"})),
            None,
        );
        recorded.bind(
            keys(&["ctrl+shift+j"]),
            "pane.split",
            Some(json!({"direction": "right"})),
            None,
        );
        assert_eq!(runs(&recorded, "ctrl+shift+d"), None);
        assert_eq!(
            runs(&recorded, "ctrl+shift+j").as_deref(),
            Some("pane.split")
        );
        // Removing twice records it once.
        recorded.remove(keys(&["ctrl+shift+d"]), "pane.split", None, None);
        assert_eq!(recorded.len(), 2);
        // A removal names its action: another action's key of the same spelling stays.
        recorded.remove(keys(&["ctrl+k"]), "pane.zoom", None, None);
        assert_eq!(runs(&recorded, "ctrl+k").as_deref(), Some("session.switch"));
        assert!(recorded.restore(&keys(&["ctrl+k"]), "pane.zoom", None));
        assert!(recorded.restore(&keys(&["ctrl+shift+d"]), "pane.split", None));
        assert!(!recorded.restore(&keys(&["ctrl+shift+d"]), "pane.split", None));
        // The keys, however they are spelled.
        recorded.remove(keys(&["Shift+Control+D"]), "pane.split", None, None);
        assert!(recorded.restore(&keys(&["ctrl+shift+d"]), "pane.split", None));
        assert_eq!(
            runs(&recorded, "ctrl+shift+d").as_deref(),
            Some("pane.split")
        );
        assert!(recorded.unbind(
            &keys(&["ctrl+shift+j"]),
            "pane.split",
            Some(&json!({"direction": "right"})),
            None
        ));
        assert!(recorded.is_empty());
    }

    #[test]
    fn a_key_recorded_twice_goes_to_the_later_action() {
        let mut recorded = Recorded::default();
        recorded.bind(keys(&["ctrl+alt+j"]), "pane.zoom", None, None);
        recorded.bind(keys(&["ctrl+alt+j"]), "client.tab.new", None, None);
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            runs(&recorded, "ctrl+alt+j").as_deref(),
            Some("client.tab.new")
        );
        // A binding of a layer below is not this layer's to forget.
        assert!(!recorded.unbind(&keys(&["ctrl+k"]), "session.switch", None, None));
        // No arguments and empty ones are one binding.
        assert!(recorded.unbind(
            &keys(&["ctrl+alt+j"]),
            "client.tab.new",
            Some(&json!({})),
            None
        ));
    }

    #[test]
    fn the_layer_sits_above_the_users_file() {
        let file = Overrides::parse(&json!([
            {"keys": "ctrl+alt+j", "action": "pane.zoom"},
            {"keys": "ctrl+alt+u", "action": "client.tab.new"},
        ]));
        let mut recorded = Recorded::default();
        recorded.bind(keys(&["ctrl+alt+j"]), "session.new", None, None);
        recorded.remove(keys(&["ctrl+alt+u"]), "client.tab.new", None, None);
        let map = keymap::resolve_all(&[defaults()], &[&file, &recorded.overrides()], &[]);
        let first = |chord: &str| {
            map.bindings
                .iter()
                .find(|bound| bound.binding.keys == keys(&[chord]))
                .map(|bound| {
                    (
                        bound.binding.action.clone(),
                        bound.binding.source.clone().unwrap_or_default(),
                    )
                })
        };
        assert_eq!(
            first("ctrl+alt+j"),
            Some(("session.new".into(), RECORDED.into()))
        );
        assert_eq!(first("ctrl+alt+u"), None);
        assert!(map.diagnostics.is_empty(), "{:?}", map.diagnostics);
        // One layer in the request, named, after the file's.
        let layer = recorded.layer();
        assert_eq!(layer.name, RECORDED);
        assert_eq!(layer.bindings.len(), 1);
        assert_eq!(layer.removals[0].action.as_deref(), Some("client.tab.new"));
    }

    #[test]
    fn older_files_load_and_the_layer_round_trips_through_the_settings_file() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(old.recorded_shortcuts.is_empty());
        // Nothing recorded writes nothing: the file is as an older version wrote it.
        assert!(
            serde_json::to_value(&old)
                .unwrap()
                .get("recorded_shortcuts")
                .is_none()
        );
        let mut settings = old;
        settings.recorded_shortcuts.bind(
            keys(&["ctrl+b", "c"]),
            "pane.split",
            Some(json!({"direction": "down"})),
            None,
        );
        settings
            .recorded_shortcuts
            .remove(keys(&["ctrl+alt+d"]), "pane.split", None, None);
        let saved = serde_json::to_value(&settings).unwrap();
        assert_eq!(
            saved["recorded_shortcuts"],
            json!([
                {"keys": ["ctrl+b", "c"], "action": "pane.split", "args": {"direction": "down"}},
                {"keys": ["ctrl+alt+d"], "action": "pane.split", "remove": true},
            ])
        );
        let back: Settings = serde_json::from_value(saved).unwrap();
        assert_eq!(back.recorded_shortcuts, settings.recorded_shortcuts);
        // An entry this version cannot read is dropped; the rest and the file still load.
        let partly: Settings = serde_json::from_value(json!({
            "recorded_shortcuts": [
                {"keys": "ctrl+j", "action": "pane.zoom"},
                {"keys": [], "action": "pane.zoom"},
                {"keys": ["ctrl+j"], "action": ""},
                {"keys": ["ctrl+j"], "action": "pane.zoom"},
                7,
            ]
        }))
        .unwrap();
        assert_eq!(partly.recorded_shortcuts.len(), 1);
        for other in [json!(null), json!("ctrl+j"), json!({"keys": ["ctrl+j"]})] {
            let settings: Settings =
                serde_json::from_value(json!({"recorded_shortcuts": other})).unwrap();
            assert!(settings.recorded_shortcuts.is_empty());
        }
    }

    #[test]
    fn named_modes_and_conditional_fallbacks_survive_editing_and_export() {
        let mut recorded = Recorded::default();
        recorded.bind(keys(&["ctrl+j"]), "pane.zoom", None, None);
        recorded.bind(keys(&["ctrl+j"]), "pane.copy", None, Some("leader".into()));
        recorded.bind(
            keys(&["ctrl+j"]),
            "client.find.next",
            Some(json!({"when":"searching"})),
            None,
        );
        assert_eq!(
            recorded.len(),
            3,
            "mode and condition are separate contexts"
        );
        recorded.remove(
            keys(&["ctrl+k"]),
            "session.switch",
            None,
            Some("leader".into()),
        );
        let settings: Settings = serde_json::from_value(recorded.as_file()).unwrap();
        assert_eq!(settings.recorded_shortcuts, recorded);
        let layer = recorded.layer();
        assert_eq!(layer.removals[0].mode.as_deref(), Some("leader"));
        assert_eq!(layer.bindings[1].mode.as_deref(), Some("leader"));
        assert!(!recorded.unbind(&keys(&["ctrl+j"]), "pane.copy", None, None));
        assert!(recorded.unbind(&keys(&["ctrl+j"]), "pane.copy", None, Some("leader")));
        assert!(!recorded.restore(&keys(&["ctrl+k"]), "session.switch", None));
        assert!(recorded.restore(&keys(&["ctrl+k"]), "session.switch", Some("leader")));
        assert_eq!(runs(&recorded, "ctrl+k").as_deref(), Some("session.switch"));
    }

    #[test]
    fn the_export_round_trips_without_losing_removal_scope() {
        let mut recorded = Recorded::default();
        recorded.bind(keys(&["ctrl+b", "c"]), "client.tab.new", None, None);
        recorded.bind(
            keys(&["ctrl+shift+j"]),
            "pane.split",
            Some(json!({"direction": "right"})),
            None,
        );
        recorded.remove(keys(&["ctrl+shift+d"]), "pane.split", None, None);
        let file = recorded.as_file();
        assert_eq!(
            file,
            json!({"recorded_shortcuts": [
                {"keys": ["ctrl+b", "c"], "action": "client.tab.new"},
                {"keys": ["ctrl+shift+j"], "action": "pane.split", "args": {"direction": "right"}},
                {"keys": ["ctrl+shift+d"], "action": "pane.split", "remove": true},
            ]})
        );
        let settings: Settings = serde_json::from_value(file).unwrap();
        assert_eq!(settings.recorded_shortcuts, recorded);
        let map = keymap::resolve(&[defaults()], &settings.recorded_shortcuts.overrides(), &[]);
        assert!(
            !map.bindings
                .iter()
                .any(|bound| bound.binding.keys == keys(&["ctrl+shift+d"]))
        );
        assert!(map.bindings.iter().any(|bound| {
            bound.binding.keys == keys(&["ctrl+b", "c"]) && bound.binding.action == "client.tab.new"
        }));
    }
}
