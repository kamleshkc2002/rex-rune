//! Local path history, keyed by saved host endpoint, never sent to Rex.
use super::Settings;
use serde::{Deserialize, Serialize};

pub const LIMIT: usize = 10;
/// The browser's maximum path length, in bytes.
pub const PATH_MAX: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentPath {
    pub path: String,
    pub directory: bool,
}

/// Controls and invisible formatting that must never be offered or pasted as paths.
/// Joiners needed by scripts and emoji remain allowed; this does not reject look-alikes.
pub fn refused(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00AD}'
                | '\u{034F}'
                | '\u{061C}'
                | '\u{115F}'
                | '\u{1160}'
                | '\u{180E}'
                | '\u{3164}'
                | '\u{FFA0}'
                | '\u{200B}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{2028}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
                | '\u{E0000}'..='\u{E007F}'
        )
}

pub fn valid(path: &str) -> bool {
    path.starts_with('/') && path.len() <= PATH_MAX && !path.chars().any(refused)
}

impl Settings {
    pub fn remember_path(&mut self, endpoint: &str, path: String, directory: bool) {
        if !self.terminal.show_recent_paths
            || !valid(&path)
            || !self.hosts.iter().any(|h| h.endpoint == endpoint)
        {
            return;
        }
        let paths = self.recent_paths.entry(endpoint.to_owned()).or_default();
        paths.retain(|r| r.path != path);
        paths.insert(0, RecentPath { path, directory });
        paths.truncate(LIMIT);
    }

    pub fn normalize_recent_paths(&mut self) {
        if !self.terminal.show_recent_paths {
            self.recent_paths.clear();
            return;
        }
        self.recent_paths.retain(|endpoint, paths| {
            if !self.hosts.iter().any(|h| h.endpoint == *endpoint) {
                return false;
            }
            let mut seen = std::collections::HashSet::new();
            paths.retain(|r| valid(&r.path) && seen.insert(r.path.clone()));
            paths.truncate(LIMIT);
            !paths.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_paths_order_dedup_cap_and_default_serialization() {
        let mut s: Settings = serde_json::from_str("{}").unwrap();
        assert!(s.terminal.show_recent_paths);
        assert!(s.recent_paths.is_empty());
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("recent_paths"));
        let endpoint = s.hosts[0].endpoint.clone();
        for i in 0..12 {
            s.remember_path(&endpoint, format!("/path/{i}"), false);
        }
        s.remember_path(&endpoint, "/path/5".into(), true);
        let paths = &s.recent_paths[&endpoint];
        assert_eq!(paths.len(), LIMIT);
        assert_eq!(
            paths[0],
            RecentPath {
                path: "/path/5".into(),
                directory: true
            }
        );
        assert_eq!(paths[1].path, "/path/11");
        assert_eq!(paths[9].path, "/path/2");
        assert_eq!(paths.iter().filter(|r| r.path == "/path/5").count(), 1);
    }

    #[test]
    fn recent_paths_save_load_drops_hostile_or_disabled_history() {
        let path =
            std::env::temp_dir().join(format!("rex-recent-paths-{}.json", std::process::id()));
        let mut s = Settings::default();
        let endpoint = s.hosts[0].endpoint.clone();
        s.remember_path(&endpoint, "/safe/Alice's folder".into(), true);
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap().recent_paths, s.recent_paths);
        for bad in [
            "relative".into(),
            format!("/{}", "x".repeat(PATH_MAX)),
            "/bad\nname".into(),
            "/bad\u{202e}name".into(),
        ] {
            s.recent_paths.get_mut(&endpoint).unwrap().push(RecentPath {
                path: bad,
                directory: true,
            });
        }
        s.recent_paths.insert(
            "ssh://removed".into(),
            vec![RecentPath {
                path: "/orphan".into(),
                directory: true,
            }],
        );
        s.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.recent_paths.len(), 1);
        assert_eq!(loaded.recent_paths[&endpoint].len(), 1);
        s.terminal.show_recent_paths = false;
        s.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert!(!loaded.terminal.show_recent_paths);
        assert!(loaded.recent_paths.is_empty());
        loaded.save(&path).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("/safe/"));
        std::fs::remove_file(path).unwrap();
    }
}
