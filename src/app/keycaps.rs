//! One spelling for every shortcut hint: menu rows, Commands, the switcher, the sidebar's search
//! chip and tooltips. Hints are read from the effective keymap, never written out by hand, so a
//! rebind shows everywhere at once and an unbound action has no hint.
use super::{theme::*, workspace::Workspace};
use rune::{El, el, row, text};
use serde_json::Value;

/// A binding's chords in order, each as its keycaps: `[["Ctrl", "B"], ["C"]]` for `ctrl+b>c`.
pub type Chords = Vec<Vec<String>>;

/// A keycap's height in a 28 pt row.
const CAP: f32 = 17.;

impl Workspace {
    /// The highest effective binding of `action` with `args`, or nothing when it is unbound
    /// (or bound only to a key this client cannot read).
    pub fn shortcut_chords(&self, action: &str, args: &Value) -> Chords {
        let empty = |args: &Value| args.is_null() || args.as_object().is_some_and(|a| a.is_empty());
        self.effective_bindings()
            .iter()
            .find(|b| {
                let bound = b.binding.args.as_ref().unwrap_or(&Value::Null);
                b.binding.action == action && (bound == args || (empty(bound) && empty(args)))
            })
            .and_then(|b| b.chords.as_ref())
            .map(|chords| chords.iter().map(|c| c.keycaps()).collect())
            .unwrap_or_default()
    }

    /// A tooltip for a control that runs `action`: its label, then the action's shortcut while
    /// it has one (`New tab · Ctrl Shift T`).
    pub fn shortcut_help(&self, label: &str, action: &str) -> String {
        let keys = spell(&self.shortcut_chords(action, &Value::Null));
        if keys.is_empty() {
            label.to_owned()
        } else {
            format!("{label} · {keys}")
        }
    }
}

/// The hint as text, for tooltips, accessible names and filtering: `Ctrl Shift D`, and
/// `Ctrl B, C` for a sequence.
pub fn spell(chords: &[Vec<String>]) -> String {
    chords
        .iter()
        .map(|caps| caps.join(" "))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One key on a chip. `lit` is for a selected row, where the hint is read next.
pub fn keycap(label: &str, lit: bool) -> El {
    el().h(CAP)
        .min_w(CAP)
        .px(4.)
        .center()
        .radius(4.5)
        .continuous_corners()
        .fill(white(if lit { KEYCAP_LIT } else { KEYCAP_FILL }))
        .border(1., white(0.06))
        .child(text(label.to_owned()).font(medium(10.5)).color(rgb(if lit {
            ink()
        } else {
            keycap_ink()
        })))
}

/// The hint as keycaps, a chip a key; the chords of a sequence are joined by a quiet "then".
pub fn keycaps(chords: &[Vec<String>], lit: bool) -> El {
    let mut caps = row().gap(2.).center_y();
    for (i, chord) in chords.iter().enumerate() {
        if i > 0 {
            caps = caps.child(text("then").font(ui(10.5)).color(rgb(muted())).mx(2.));
        }
        for cap in chord {
            caps = caps.child(keycap(cap, lit));
        }
    }
    caps
}
