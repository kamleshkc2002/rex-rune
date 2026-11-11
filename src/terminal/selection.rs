//! Native tracked anchors keep selection attached to content, not viewport row numbers.
use anyhow::Result;
use ghostty_vt::{
    Terminal,
    error::Error,
    selection::{
        FormatOptions,
        gesture::{
            Autoscroll, AutoscrollTickEvent, Behavior, Behaviors, DragEvent, Gesture, PressEvent,
            ReleaseEvent,
        },
    },
    terminal::{Point, PointCoordinate, ScrollViewport},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub use ghostty_vt::selection::gesture::Geometry;

pub const COPY_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub enum Phase {
    Down(u32),
    Move,
    Up,
    Clear,
    /// Select everything the terminal holds, scrollback included; `at`, `position` and
    /// `geometry` are unused.
    All,
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub generation: u64,
    pub revision: u64,
    pub phase: Phase,
    pub at: (u16, u16),
    pub position: (f64, f64),
    pub geometry: Geometry,
}

pub struct Controller {
    gesture: Gesture,
    press: PressEvent,
    drag: DragEvent,
    release: ReleaseEvent,
    tick: AutoscrollTickEvent,
    pointer: Option<Input>,
    screen: Option<i32>,
    last_tick: Instant,
    pub revision: u64,
}

impl Controller {
    pub fn new() -> Result<Self> {
        // Match the client's word/path selection: shell path separators stay inside words.
        let boundaries: Vec<char> = " \t\r\n!\"#$&'()*,;<>?[]^`{|}".chars().collect();
        let mut press = PressEvent::new()?;
        press.set_word_boundary_codepoints(&boundaries)?;
        let mut drag = DragEvent::new()?;
        drag.set_word_boundary_codepoints(&boundaries)?;
        let mut tick = AutoscrollTickEvent::new()?;
        tick.set_word_boundary_codepoints(&boundaries)?;
        Ok(Self {
            gesture: Gesture::new()?,
            press,
            drag,
            release: ReleaseEvent::new()?,
            tick,
            pointer: None,
            screen: None,
            last_tick: Instant::now(),
            revision: 0,
        })
    }

    fn point(terminal: &Terminal, input: Input) -> Result<PointCoordinate> {
        Ok(PointCoordinate {
            x: input.at.0.min(terminal.cols()?.saturating_sub(1)),
            y: u32::from(input.at.1.min(terminal.rows()?.saturating_sub(1))),
        })
    }

    pub fn apply(&mut self, terminal: &Terminal, input: Input) -> Result<()> {
        self.revision = input.revision;
        let screen = terminal.active_screen()? as i32;
        if self.screen != Some(screen) {
            self.gesture.reset(terminal);
            self.pointer = None;
            terminal.set_selection(None)?;
            self.screen = Some(screen);
            if !matches!(input.phase, Phase::Down(_) | Phase::All) {
                return Ok(());
            }
        }
        if matches!(input.phase, Phase::Clear | Phase::All) {
            self.gesture.reset(terminal);
            self.pointer = None;
            let all = match input.phase {
                Phase::All => terminal.select_all()?,
                _ => None,
            };
            terminal.set_selection(all.as_ref())?;
            return Ok(());
        }
        let at = terminal.grid_ref(Point::Viewport(Self::point(terminal, input)?))?;
        match input.phase {
            Phase::Down(clicks) => {
                self.gesture.reset(terminal);
                let behavior = match clicks {
                    0 | 1 => Behavior::Cell,
                    2 => Behavior::Word,
                    _ => Behavior::Line,
                };
                self.press
                    .set_behaviors(&Behaviors::new().with_single_click_behavior(behavior))?
                    .set_position(input.position.0, input.position.1)?;
                let selected = self.press.apply(&mut self.gesture, terminal, at)?;
                terminal.set_selection(selected.as_ref())?;
                self.pointer = Some(input);
            }
            Phase::Move => {
                if self.pointer.is_some() {
                    self.drag.set_position(input.position.0, input.position.1)?;
                    let selected =
                        self.drag
                            .apply(&mut self.gesture, terminal, at, input.geometry)?;
                    terminal.set_selection(selected.as_ref())?;
                    self.pointer = Some(input);
                }
            }
            Phase::Up => {
                self.release.apply(&mut self.gesture, terminal, Some(at))?;
                self.pointer = None;
            }
            Phase::Clear | Phase::All => unreachable!(),
        }
        Ok(())
    }

    pub fn after_scroll(&mut self, terminal: &Terminal) -> Result<()> {
        if let Some(mut input) = self.pointer {
            input.phase = Phase::Move;
            self.apply(terminal, input)?;
        }
        Ok(())
    }

    pub fn stop(&mut self, terminal: &Terminal) -> Result<()> {
        if self.pointer.take().is_some() {
            if self.screen == Some(terminal.active_screen()? as i32) {
                self.release.apply(&mut self.gesture, terminal, None)?;
            } else {
                self.gesture.reset(terminal);
            }
        }
        Ok(())
    }

    pub fn autoscrolling(&self, terminal: &Terminal) -> bool {
        self.pointer.is_some()
            && terminal
                .active_screen()
                .ok()
                .is_some_and(|s| self.screen == Some(s as i32))
            && matches!(
                self.gesture.autoscroll(terminal),
                Ok(Autoscroll::Up | Autoscroll::Down)
            )
    }

    pub fn tick(&mut self, terminal: &mut Terminal) -> Result<bool> {
        if self.screen != Some(terminal.active_screen()? as i32) {
            self.stop(terminal)?;
            return Ok(false);
        }
        let Some(input) = self.pointer else {
            return Ok(false);
        };
        if self.last_tick.elapsed() < Duration::from_millis(50) {
            return Ok(false);
        }
        let delta = match self.gesture.autoscroll(terminal)? {
            Autoscroll::Up => -1,
            Autoscroll::Down => 1,
            _ => return Ok(false),
        };
        terminal.scroll_viewport(ScrollViewport::Delta(delta));
        let at = Self::point(terminal, input)?;
        self.tick.set_position(input.position.0, input.position.1)?;
        let selected = self
            .tick
            .apply(&mut self.gesture, terminal, at, input.geometry)?;
        terminal.set_selection(selected.as_ref())?;
        self.last_tick = Instant::now();
        Ok(true)
    }
}

/// Reuse a bounded formatting buffer; copying a large selection cannot allocate without limit.
pub fn text(terminal: &Terminal, buffer: &mut Vec<u8>) -> Result<Option<Arc<str>>> {
    let options = || FormatOptions::new().with_unwrap(true).with_trim(true);
    if buffer.is_empty() {
        buffer.resize(4096, 0)
    }
    let length = match terminal.format_selection_buf(options(), buffer) {
        Err(Error::OutOfSpace { required }) if required <= COPY_LIMIT => {
            buffer.resize(required, 0);
            terminal.format_selection_buf(options(), buffer)?
        }
        Err(Error::OutOfSpace { .. }) => anyhow::bail!("Selection exceeds the 8 MiB copy limit"),
        result => result?,
    };
    length
        .map(|n| {
            std::str::from_utf8(&buffer[..n])
                .map(Arc::from)
                .map_err(Into::into)
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(phase: Phase, x: u16, y: u16) -> Input {
        Input {
            generation: 0,
            revision: 1,
            phase,
            at: (x, y),
            position: (f64::from(x) * 10., f64::from(y) * 20.),
            geometry: Geometry {
                columns: 32,
                cell_width: 10,
                padding_left: 0,
                screen_height: 100,
            },
        }
    }

    #[test]
    fn selected_text_is_pinned_across_viewport_scrolls_and_output() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 32,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        for n in 0..30 {
            terminal.vt_write(format!("line-{n:02} 世界 e\u{301}\r\n").as_bytes());
        }
        terminal.scroll_viewport(ScrollViewport::Delta(-8));
        let mut selection = Controller::new().unwrap();
        selection
            .apply(&terminal, input(Phase::Down(1), 0, 1))
            .unwrap();
        selection
            .apply(&terminal, input(Phase::Move, 20, 1))
            .unwrap();
        selection.apply(&terminal, input(Phase::Up, 20, 1)).unwrap();
        let mut buffer = Vec::new();
        let original = text(&terminal, &mut buffer).unwrap().unwrap();
        assert!(original.contains("世界 e\u{301}"), "{original}");
        terminal.scroll_viewport(ScrollViewport::Delta(-3));
        assert_eq!(text(&terminal, &mut buffer).unwrap().unwrap(), original);
        terminal.vt_write(b"new output\r\n");
        assert_eq!(text(&terminal, &mut buffer).unwrap().unwrap(), original);
        selection
            .apply(&terminal, input(Phase::Clear, 0, 0))
            .unwrap();
        assert!(text(&terminal, &mut buffer).unwrap().is_none());
    }

    #[test]
    fn native_gestures_keep_words_paths_lines_and_focus_click_semantics() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 40,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        terminal.vt_write("alpha /tmp/beta-gamma.txt 世界\r\ne\u{301} 🦀 tail".as_bytes());
        let mut selection = Controller::new().unwrap();
        let mut buffer = Vec::new();
        selection
            .apply(&terminal, input(Phase::Down(1), 12, 0))
            .unwrap();
        selection.apply(&terminal, input(Phase::Up, 12, 0)).unwrap();
        assert!(text(&terminal, &mut buffer).unwrap().is_none());
        selection
            .apply(&terminal, input(Phase::Down(2), 12, 0))
            .unwrap();
        assert_eq!(
            &*text(&terminal, &mut buffer).unwrap().unwrap(),
            "/tmp/beta-gamma.txt"
        );
        selection
            .apply(&terminal, input(Phase::Down(3), 3, 1))
            .unwrap();
        assert_eq!(
            &*text(&terminal, &mut buffer).unwrap().unwrap(),
            "e\u{301} 🦀 tail"
        );
    }
}

#[cfg(test)]
mod select_all_tests {
    use super::*;

    fn input(phase: Phase) -> Input {
        Input {
            generation: 0,
            revision: 1,
            phase,
            at: (0, 0),
            position: (0., 0.),
            geometry: Geometry {
                columns: 20,
                cell_width: 10,
                padding_left: 0,
                screen_height: 100,
            },
        }
    }

    #[test]
    fn select_all_takes_the_screen_and_scrollback_and_a_click_replaces_it() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        let mut selection = Controller::new().unwrap();
        let mut buffer = Vec::new();
        selection.apply(&terminal, input(Phase::All)).unwrap();
        assert!(
            text(&terminal, &mut buffer)
                .unwrap()
                .is_none_or(|t| t.is_empty()),
            "an empty terminal has nothing to select"
        );
        for n in 0..40 {
            terminal.vt_write(format!("line-{n:02}\r\n").as_bytes());
        }
        terminal.vt_write(b"tail");
        selection.apply(&terminal, input(Phase::All)).unwrap();
        let all = text(&terminal, &mut buffer).unwrap().unwrap();
        assert!(all.starts_with("line-00\nline-01"), "{all:?}");
        assert!(all.ends_with("line-39\ntail"), "{all:?}");
        assert_eq!(all.lines().count(), 41);
        // Scrolling leaves it attached to its content.
        terminal.scroll_viewport(ScrollViewport::Delta(-12));
        assert_eq!(text(&terminal, &mut buffer).unwrap().unwrap(), all);
        // A stray release does nothing; a click starts over.
        selection.apply(&terminal, input(Phase::Up)).unwrap();
        assert_eq!(text(&terminal, &mut buffer).unwrap().unwrap(), all);
        selection.apply(&terminal, input(Phase::Down(1))).unwrap();
        selection.apply(&terminal, input(Phase::Up)).unwrap();
        assert!(text(&terminal, &mut buffer).unwrap().is_none());
        selection.apply(&terminal, input(Phase::All)).unwrap();
        selection.apply(&terminal, input(Phase::Clear)).unwrap();
        assert!(text(&terminal, &mut buffer).unwrap().is_none());
    }
}

#[cfg(test)]
mod scrolling_tests {
    use super::*;
    #[test]
    fn edge_drag_autoscroll_extends_then_release_stops_it() {
        let mut terminal = Terminal::new(ghostty_vt::TerminalOptions {
            cols: 20,
            rows: 5,
            ..Default::default()
        })
        .unwrap();
        for n in 0..40 {
            terminal.vt_write(format!("line-{n:02}\r\n").as_bytes());
        }
        let mut controller = Controller::new().unwrap();
        let mut event = Input {
            generation: 0,
            revision: 1,
            phase: Phase::Down(1),
            at: (0, 2),
            position: (0., 40.),
            geometry: Geometry {
                columns: 20,
                cell_width: 10,
                padding_left: 0,
                screen_height: 100,
            },
        };
        controller.apply(&terminal, event).unwrap();
        event.phase = Phase::Move;
        event.at = (0, 0);
        event.position = (0., -20.);
        controller.apply(&terminal, event).unwrap();
        assert!(controller.autoscrolling(&terminal));
        let mut buffer = Vec::new();
        let before = text(&terminal, &mut buffer).unwrap().unwrap();
        controller.last_tick = Instant::now() - Duration::from_millis(60);
        assert!(controller.tick(&mut terminal).unwrap());
        let after = text(&terminal, &mut buffer).unwrap().unwrap();
        assert!(
            after.lines().count() > before.lines().count(),
            "{before:?} -> {after:?}"
        );
        event.phase = Phase::Up;
        controller.apply(&terminal, event).unwrap();
        assert!(!controller.autoscrolling(&terminal));
        assert!(!controller.tick(&mut terminal).unwrap());
    }
}
