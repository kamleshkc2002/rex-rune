//! Types and functions around terminal state management.
//!
//! Adapted from libghostty-vt 0.2.2 (MIT OR Apache-2.0).

use std::{
    ffi::c_void,
    io::Write,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::{
    error::{Error, Result, from_optional_result_uninit, from_result, from_result_with_len},
    ffi::{self, TerminalData as Data, TerminalOption as Opt},
    key,
    screen::{GridRef, Screen, TrackedGridRef},
    style::{self, Palette, RawPalette, RgbColor},
};

#[doc(inline)]
pub use ffi::{SizeReportSize, TerminalScrollbar as Scrollbar};

/// Complete terminal emulator state and rendering.
///
/// A terminal instance manages the full emulator state including the screen,
/// scrollback, cursor, styles, modes, and VT stream processing.
///
/// Once a terminal session is up and running, you can configure a key encoder
/// to write keyboard input via [`key::Encoder::set_options_from_terminal`].
///
/// ## Example: VT stream processing
///
/// ```
/// use ghostty_vt::{Terminal, TerminalOptions};
///
/// // Create a terminal
/// let mut terminal = Terminal::new(TerminalOptions {
///     cols: 80,
///     rows: 24,
///     ..Default::default()
/// }).unwrap();
///
/// // Feed VT data into the terminal
/// terminal.vt_write(b"Hello, World!\r\n");
///
/// // ANSI color codes: ESC[1;32m = bold green, ESC[0m = reset
/// terminal.vt_write(b"\x1b[1;32mGreen Text\x1b[0m\r\n");
/// ```
///
/// # Events
///
/// By default, the terminal sequence processing with [`Terminal::vt_write`]
/// only processes sequences that directly affect terminal state. Sequences
/// that have side effects or require responses (bell characters, title
/// changes, device attribute queries, clipboard writes, and more) are
/// reported as [`Event`] values. The terminal queues them during
/// [`Terminal::vt_write`], and the owner drains the queue with
/// [`Terminal::events`] afterwards.
///
/// Queries that need an answer while the stream is being parsed (device
/// attributes, XTVERSION, ENQ, size reports, color scheme reports) are
/// answered from values configured on the terminal, such as
/// [`Terminal::set_device_attributes`]. The answers go to the pty through
/// [`Event::PtyWrite`].
///
/// ```rust
/// use ghostty_vt::{Event, Terminal, TerminalOptions};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut terminal = Terminal::new(TerminalOptions { cols: 80, rows: 24, ..Default::default() })?;
///
/// // Bell, title change, DECRQM query, bell.
/// terminal.vt_write(b"\x07\x1b]2;Hello Events\x1b\\\x1B[?7$p\x07");
///
/// let mut bells = 0;
/// for event in terminal.events() {
///     match event {
///         Event::Bell => bells += 1,
///         Event::TitleChanged(title) => assert_eq!(title, "Hello Events"),
///         Event::PtyWrite(bytes) => assert!(!bytes.is_empty()),
///         _ => {}
///     }
/// }
/// assert_eq!(bells, 2);
/// # Ok(())}
/// ```
///
/// # Color theme
///
/// The terminal maintains a set of colors used for rendering: a foreground
/// color, a background color, a cursor color, and a 256-color palette. Each
/// of these has two layers: a **default** value set by the embedder, and an
/// **override** value that programs running in the terminal can set via OSC
/// escape sequences (e.g. OSC 10/11/12 for foreground/background/cursor,
/// OSC 4 for individual palette entries).
///
/// Use [`Terminal::set_default_fg_color`], [`Terminal::set_default_bg_color`],
/// [`Terminal::set_default_cursor_color`] and [`Terminal::set_default_color_palette`]
/// to configure the default colors. The getters come in two variants: the
/// **effective** value (the OSC override if one is active, otherwise the
/// default) and the **default** value (ignoring any OSC override).
#[derive(Debug)]
pub struct Terminal {
    pub(crate) inner: NonNull<ffi::TerminalImpl>,
    // Heap state the C callbacks write into. It never moves, so the userdata
    // pointer stays valid while the Terminal itself may move.
    shared: NonNull<Shared>,
}

/// Terminal initialization options.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Terminal width in cells. Must be greater than zero.
    pub cols: u16,
    /// Terminal height in cells. Must be greater than zero.
    pub rows: u16,
    /// Maximum scrollback size in bytes. Zero disables scrollback.
    ///
    /// This is an estimate, pruned at page granularity. See
    /// [`Terminal::set_scrollback_max_bytes`].
    pub max_scrollback: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            // Ghostty's default `scrollback-limit`.
            max_scrollback: 10_000_000,
        }
    }
}

/// Default visual style used when the cursor style is reset.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, int_enum::IntEnum)]
#[non_exhaustive]
pub enum CursorStyle {
    /// Bar cursor (DECSCUSR 5, 6).
    Bar = ffi::TerminalCursorStyle::BAR,
    /// Block cursor (DECSCUSR 1, 2).
    Block = ffi::TerminalCursorStyle::BLOCK,
    /// Underline cursor (DECSCUSR 3, 4).
    Underline = ffi::TerminalCursorStyle::UNDERLINE,
    /// Hollow block cursor.
    BlockHollow = ffi::TerminalCursorStyle::BLOCK_HOLLOW,
}

/// A render hold callback. See [`Terminal::set_render_hold`].
pub type RenderHoldFn = Box<dyn FnMut(&Terminal, bool)>;

/// State shared between the terminal handle and the C callbacks.
struct Shared {
    events: Vec<Event>,
    device_attributes: Option<ffi::DeviceAttributes>,
    xtversion: Option<String>,
    enquiry: Option<Vec<u8>>,
    color_scheme: Option<ColorScheme>,
    size: Option<SizeReportSize>,
    size_reports: bool,
    clipboard_write_policy: ClipboardWritePolicy,
    render_hold: Option<RenderHoldFn>,
}

impl Terminal {
    /// Create a new terminal instance.
    ///
    /// The terminal starts with Ghostty's defaults: grapheme clustering
    /// (mode 2027) on, Ghostty's device attributes, size reports on, and
    /// clipboard writes allowed.
    pub fn new(opts: Options) -> Result<Self> {
        let mut raw: ffi::Terminal = std::ptr::null_mut();
        let result = unsafe {
            ffi::ghostty_terminal_new(std::ptr::null(), &raw mut raw, opts.cols, opts.rows)
        };
        from_result(result)?;
        // SAFETY: A successful ghostty_terminal_new returns a valid, owned terminal.
        let mut terminal = unsafe { Self::from_raw(raw) }?;
        terminal.set_scrollback_max_bytes(Some(opts.max_scrollback))?;
        terminal.set_mode_default(Mode::GRAPHEME_CLUSTER, true)?;
        Ok(terminal)
    }

    /// Wrap an owned raw terminal and install the callbacks.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid terminal that nothing else owns.
    pub(crate) unsafe fn from_raw(raw: ffi::Terminal) -> Result<Self> {
        let inner = NonNull::new(raw).ok_or(Error::OutOfMemory)?;
        let shared = Box::new(Shared {
            events: Vec::new(),
            device_attributes: Some(DeviceAttributes::default().into()),
            xtversion: None,
            enquiry: None,
            color_scheme: None,
            size: None,
            size_reports: true,
            clipboard_write_policy: ClipboardWritePolicy::Allow,
            render_hold: None,
        });
        let shared = NonNull::from(Box::leak(shared));
        let terminal = Self { inner, shared };
        terminal.set_ptr(Opt::USERDATA, shared.as_ptr().cast_const().cast())?;
        terminal.install_callbacks()?;
        Ok(terminal)
    }

    pub(crate) fn as_raw(&self) -> ffi::Terminal {
        self.inner.as_ptr()
    }

    fn shared(&mut self) -> &mut Shared {
        // SAFETY: `shared` is a leaked Box owned by this terminal. C only
        // touches it during vt_write, which takes `&mut self` and therefore
        // cannot overlap with this borrow.
        unsafe { self.shared.as_mut() }
    }

    fn shared_ref(&self) -> &Shared {
        // SAFETY: See `shared`.
        unsafe { self.shared.as_ref() }
    }

    /// Write VT-encoded data to the terminal for processing.
    ///
    /// Feeds raw bytes through the terminal's VT stream parser, updating
    /// terminal state accordingly. Side effects and query responses are
    /// queued as [`Event`]s; drain them with [`Terminal::events`].
    ///
    /// This never fails. Any erroneous input or errors in processing the input
    /// are logged internally but do not cause this function to fail because
    /// this input is assumed to be untrusted and from an external source; so
    /// the primary goal is to keep the terminal state consistent and not allow
    /// malformed input to corrupt or crash.
    pub fn vt_write(&mut self, data: &[u8]) {
        unsafe { ffi::ghostty_terminal_vt_write(self.inner.as_ptr(), data.as_ptr(), data.len()) }
    }

    /// Write VT-encoded data, but only the shortest prefix needed to reach ground.
    ///
    /// Ground is when the stream isn't in the middle of any type of sequence:
    /// UTF-8, ESC, CSI, OSC, etc. It is the point at which out-of-band VT
    /// sequences can be inserted safely.
    ///
    /// Returns `Ok(Some(consumed))` when ground was reached, where `consumed`
    /// includes the byte that reached it (zero when the stream was already at
    /// ground), or `Ok(None)` when the full slice was consumed without reaching
    /// ground.
    pub fn vt_write_until_ground(&mut self, data: &[u8]) -> Result<Option<usize>> {
        let mut stop = ffi::TerminalWriteStop {
            size: std::mem::size_of::<ffi::TerminalWriteStop>(),
            ..Default::default()
        };
        let result = unsafe {
            ffi::ghostty_terminal_vt_write_until(
                self.inner.as_ptr(),
                data.as_ptr(),
                data.len(),
                ffi::TerminalBoundary::GROUND as u32,
                &raw mut stop,
            )
        };
        crate::error::from_optional_result_with_len(result, stop.consumed)
    }

    /// Drain the events queued since the last drain.
    ///
    /// Events are queued in the order the VT stream produced them.
    pub fn events(&mut self) -> std::vec::Drain<'_, Event> {
        self.shared().events.drain(..)
    }

    /// Whether events are waiting to be drained.
    #[must_use]
    pub fn has_events(&self) -> bool {
        !self.shared_ref().events.is_empty()
    }

    /// Resize the terminal to the given dimensions.
    ///
    /// Changes the number of columns and rows in the terminal. The primary
    /// screen will reflow content if wraparound mode is enabled; the alternate
    /// screen does not reflow. If the dimensions are unchanged, this is a no-op.
    ///
    /// This also updates the terminal's pixel dimensions (used for image
    /// protocols and size reports), disables synchronized output mode (allowed
    /// by the spec so that resize results are shown immediately), and sends an
    /// in-band size report if mode 2048 is enabled.
    ///
    /// The geometry is remembered and used to answer size queries
    /// (CSI 14/16/18 t) while [size reports](Terminal::set_size_reports)
    /// are enabled.
    pub fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> Result<()> {
        self.shared().size = Some(SizeReportSize {
            rows,
            columns: cols,
            cell_width: cell_width_px,
            cell_height: cell_height_px,
        });
        let result = unsafe {
            ffi::ghostty_terminal_resize(
                self.inner.as_ptr(),
                cols,
                rows,
                cell_width_px,
                cell_height_px,
            )
        };
        from_result(result)
    }

    /// Perform a full reset of the terminal (RIS).
    ///
    /// Resets all terminal state back to its initial configuration,
    /// including modes, scrollback, scrolling region, and screen contents.
    /// The terminal dimensions are preserved.
    pub fn reset(&mut self) {
        unsafe { ffi::ghostty_terminal_reset(self.inner.as_ptr()) }
    }

    /// Scroll the terminal viewport.
    pub fn scroll_viewport(&mut self, scroll: ScrollViewport) {
        unsafe { ffi::ghostty_terminal_scroll_viewport(self.inner.as_ptr(), scroll.into()) }
    }

    /// Resolve a point in the terminal grid to a grid reference.
    ///
    /// Resolves the given point (which can be in active, viewport, screen,
    /// or history coordinates) to a grid reference for that location. Use
    /// [`GridRef::cell`] and [`GridRef::row`] to extract the cell and row.
    ///
    /// Lookups in the active region and viewport are fast. Lookups in the
    /// screen and history may require traversing the full scrollback page
    /// list to resolve the y coordinate, so they can be expensive for large
    /// scrollback buffers.
    ///
    /// This function isn't meant to be used as the core of render loop.
    /// Use the [render state API](crate::render::RenderState) for that.
    pub fn grid_ref(&self, point: Point) -> Result<GridRef<'_>> {
        let mut grid_ref = ffi::sized!(ffi::GridRef);
        let result = unsafe {
            ffi::ghostty_terminal_grid_ref(self.inner.as_ptr(), point.into(), &raw mut grid_ref)
        };
        from_result(result)?;
        Ok(unsafe { GridRef::from_raw(grid_ref) })
    }

    /// Create an owned tracked grid reference for a terminal point.
    ///
    /// This is the tracked variant of [`Terminal::grid_ref`]. The returned handle
    /// follows the referenced cell as the terminal's page list is modified:
    /// scrolling, pruning, resize/reflow, and other page-list operations update
    /// the tracked reference automatically.
    ///
    /// If the point is outside the requested coordinate space, this returns
    /// `Err(Error::InvalidValue)`.
    ///
    /// If the tracked grid reference outlives this terminal, the handle remains
    /// valid, but it will always return `false` or `Ok(None)`.
    pub fn track_grid_ref(&self, point: Point) -> Result<TrackedGridRef> {
        let mut raw: ffi::TrackedGridRef = std::ptr::null_mut();
        let result = unsafe {
            ffi::ghostty_terminal_grid_ref_track(self.inner.as_ptr(), point.into(), &raw mut raw)
        };
        from_result(result)?;

        let inner = NonNull::new(raw).ok_or(Error::InvalidValue)?;
        Ok(TrackedGridRef::new(inner, self.inner))
    }

    /// Convert a grid reference back to a point in the given coordinate system.
    ///
    /// This is the inverse of [`Terminal::grid_ref`]: given a grid reference, it
    /// returns the x/y coordinates in the requested coordinate system (active,
    /// viewport, screen, or history).
    ///
    /// Not every grid reference is representable in every coordinate system.
    /// For example, a cell in scrollback history cannot be expressed in active
    /// coordinates, and a cell that has scrolled off the visible area cannot
    /// be expressed in viewport coordinates. In these cases, the function
    /// returns `Ok(None)`.
    pub fn point_from_grid_ref(
        &self,
        grid_ref: &GridRef<'_>,
        space: PointSpace,
    ) -> Result<Option<PointCoordinate>> {
        let mut point = MaybeUninit::<ffi::PointCoordinate>::zeroed();
        let result = unsafe {
            ffi::ghostty_terminal_point_from_grid_ref(
                self.inner.as_ptr(),
                std::ptr::from_ref(&grid_ref.inner),
                space.into_raw(),
                point.as_mut_ptr(),
            )
        };

        from_optional_result_uninit(result, point).map(|value| value.map(Into::into))
    }

    /// Get the current value of a terminal mode.
    pub fn mode(&self, mode: Mode) -> Result<bool> {
        let mut config = ffi::TerminalModeConfig {
            mode: mode.0,
            value: false,
        };
        let result = unsafe {
            ffi::ghostty_terminal_get(
                self.inner.as_ptr(),
                Data::MODE,
                std::ptr::from_mut(&mut config).cast(),
            )
        };
        from_result(result)?;
        Ok(config.value)
    }

    /// Set the current value of a terminal mode.
    ///
    /// This does not change the value restored by a full terminal reset.
    /// Changing a mode this way never queues an event, so ending a render
    /// hold by resetting mode 2026 here does not produce
    /// [`Event::RenderHold`].
    pub fn set_mode(&mut self, mode: Mode, value: bool) -> Result<&mut Self> {
        let config = ffi::TerminalModeConfig {
            mode: mode.0,
            value,
        };
        self.set(Opt::MODE, &config)?;
        Ok(self)
    }

    /// Set the reset default for a terminal mode.
    ///
    /// This updates both the current value and the value restored by a full
    /// terminal reset (RIS). Modes that mirror other terminal state cannot
    /// be configured this way and return [`Error::InvalidValue`].
    pub fn set_mode_default(&mut self, mode: Mode, value: bool) -> Result<&mut Self> {
        let config = ffi::TerminalModeConfig {
            mode: mode.0,
            value,
        };
        self.set(Opt::MODE_DEFAULT, &config)?;
        Ok(self)
    }

    /// Compress eligible terminal scrollback.
    ///
    /// Incremental mode performs bounded work suitable for an idle callback.
    /// A pending result means the application should invoke another step while
    /// the terminal remains idle. A complete result means no continuation is
    /// needed until `Terminal::compression_activity` changes. Full mode
    /// performs one synchronous scan and can stall on large scrollback buffers.
    pub fn compress(&mut self, mode: CompressionMode) -> Result<CompressionResult> {
        let mut value = ffi::TerminalCompressionResult::UNSUPPORTED;
        let result = unsafe {
            ffi::ghostty_terminal_compress(self.inner.as_ptr(), mode.into(), &raw mut value)
        };
        from_result(result)?;
        value.try_into().map_err(|_| Error::InvalidValue)
    }

    /// Return the current compression activity token.
    ///
    /// The token is opaque and only equality comparisons are meaningful.
    /// An embedding application should cache it and restart its compression
    /// idle delay whenever the value changes.
    pub fn compression_activity(&self) -> Result<CompressionActivity> {
        let mut value = 0;
        let result = unsafe {
            ffi::ghostty_terminal_compression_activity(self.inner.as_ptr(), &raw mut value)
        };
        from_result(result)?;
        Ok(CompressionActivity(value))
    }

    pub(crate) fn get<T>(&self, tag: ffi::TerminalData::Type) -> Result<T> {
        let mut value = MaybeUninit::<T>::zeroed();
        let result = unsafe {
            ffi::ghostty_terminal_get(self.inner.as_ptr(), tag, value.as_mut_ptr().cast())
        };
        from_result(result)?;
        // SAFETY: Value should be initialized after successful call.
        Ok(unsafe { value.assume_init() })
    }
    pub(crate) fn get_optional<T>(&self, tag: ffi::TerminalData::Type) -> Result<Option<T>> {
        let mut value = MaybeUninit::<T>::zeroed();
        let result = unsafe {
            ffi::ghostty_terminal_get(self.inner.as_ptr(), tag, value.as_mut_ptr().cast())
        };
        from_optional_result_uninit(result, value)
    }
    pub(crate) fn set<T>(&self, tag: ffi::TerminalOption::Type, v: &T) -> Result<()> {
        let result = unsafe {
            ffi::ghostty_terminal_set(self.inner.as_ptr(), tag, std::ptr::from_ref(v).cast())
        };
        from_result(result)
    }
    /// Set an option whose ABI expects the pointer value itself, not a pointer
    /// to Rust storage containing that value.
    pub(crate) fn set_ptr(&self, tag: ffi::TerminalOption::Type, ptr: *const c_void) -> Result<()> {
        let result = unsafe { ffi::ghostty_terminal_set(self.inner.as_ptr(), tag, ptr) };
        from_result(result)
    }
    pub(crate) fn set_optional<T>(
        &self,
        tag: ffi::TerminalOption::Type,
        v: Option<&T>,
    ) -> Result<()> {
        let ptr = if let Some(v) = v {
            std::ptr::from_ref(v)
        } else {
            std::ptr::null()
        };

        let result = unsafe { ffi::ghostty_terminal_set(self.inner.as_ptr(), tag, ptr.cast()) };
        from_result(result)
    }

    /// Get the terminal width in cells.
    pub fn cols(&self) -> Result<u16> {
        self.get(Data::COLS)
    }
    /// Get the terminal height in cells.
    pub fn rows(&self) -> Result<u16> {
        self.get(Data::ROWS)
    }
    /// Get the terminal width in pixels, as set by the last resize.
    pub fn width_px(&self) -> Result<u32> {
        self.get(Data::WIDTH_PX)
    }
    /// Get the terminal height in pixels, as set by the last resize.
    pub fn height_px(&self) -> Result<u32> {
        self.get(Data::HEIGHT_PX)
    }
    /// Get the cursor column position (zero-indexed).
    pub fn cursor_x(&self) -> Result<u16> {
        self.get(Data::CURSOR_X)
    }
    /// Get the cursor row position within the active area (zero-indexed).
    pub fn cursor_y(&self) -> Result<u16> {
        self.get(Data::CURSOR_Y)
    }
    /// Get whether the cursor has a pending wrap (next print will soft-wrap).
    pub fn is_cursor_pending_wrap(&self) -> Result<bool> {
        self.get(Data::CURSOR_PENDING_WRAP)
    }
    /// Get whether the cursor is visible (DEC mode 25).
    pub fn is_cursor_visible(&self) -> Result<bool> {
        self.get(Data::CURSOR_VISIBLE)
    }
    /// Get whether the cursor is at a semantic shell prompt or input area
    /// (OSC 133). False when no prompt information is available or the
    /// alternate screen is active.
    pub fn is_cursor_at_prompt(&self) -> Result<bool> {
        self.get(Data::CURSOR_AT_PROMPT)
    }
    /// Get the current SGR style of the cursor.
    ///
    /// This is the style that will be applied to newly printed characters.
    pub fn cursor_style(&self) -> Result<style::Style> {
        self.get::<ffi::Style>(Data::CURSOR_STYLE)
            .and_then(std::convert::TryInto::try_into)
    }
    /// Get the current Kitty keyboard protocol flags.
    pub fn kitty_keyboard_flags(&self) -> Result<key::KittyKeyFlags> {
        self.get::<ffi::KittyKeyFlags>(Data::KITTY_KEYBOARD_FLAGS)
            .map(key::KittyKeyFlags::from_bits_retain)
    }

    /// Get the scrollbar state for the terminal viewport.
    ///
    /// This may be expensive to calculate depending on where the viewport is
    /// (arbitrary pins are expensive). The caller should take care to only call
    /// this as needed and not too frequently.
    pub fn scrollbar(&self) -> Result<Scrollbar> {
        self.get(Data::SCROLLBAR)
    }
    /// Get whether the viewport is pinned to the active area, that is,
    /// not scrolled into history.
    pub fn is_viewport_active(&self) -> Result<bool> {
        self.get(Data::VIEWPORT_ACTIVE)
    }
    /// Get the currently active screen.
    pub fn active_screen(&self) -> Result<Screen> {
        self.get::<ffi::TerminalScreen::Type>(Data::ACTIVE_SCREEN)
            .and_then(|v| v.try_into().map_err(|_| Error::InvalidValue))
    }
    /// Get whether any mouse tracking mode is active.
    ///
    /// Returns true if any of the mouse tracking modes (X10, normal, button,
    /// or any-event) are enabled.
    pub fn is_mouse_tracking(&self) -> Result<bool> {
        self.get(Data::MOUSE_TRACKING)
    }
    /// Get whether VT processing is at ground, that is, not in the middle of
    /// any sequence. Out-of-band VT sequences can be inserted safely there.
    pub fn is_vt_ground(&self) -> Result<bool> {
        self.get(Data::VT_GROUND)
    }
    /// Get whether VT processing encountered a non-gracefully handled error
    /// at some point. This is informational and is never cleared.
    pub fn has_vt_processing_error(&self) -> Result<bool> {
        self.get(Data::VT_PROCESSING_ERROR)
    }
    /// Get the terminal title as set by escape sequences (e.g. OSC 0/2).
    ///
    /// Returns a borrowed string, valid until the next call to
    /// [`Terminal::vt_write`] or [`Terminal::reset`]. An empty string is
    /// returned when no title has been set.
    pub fn title(&self) -> Result<&str> {
        let str = self.get::<ffi::String>(Data::TITLE)?;
        // SAFETY: We trust libghostty to return a valid borrowed string,
        // while we uphold that no mutation could happen during its lifetime.
        let str = unsafe { str.to_bytes() };
        std::str::from_utf8(str).map_err(|_| Error::InvalidValue)
    }

    /// Get the current working directory as set by escape sequences (e.g. OSC 7).
    ///
    /// Returns a borrowed string, valid until the next call to
    /// [`Terminal::vt_write`] or [`Terminal::reset`]. An empty string is
    /// returned when no directory has been set.
    pub fn pwd(&self) -> Result<&str> {
        let str = self.get::<ffi::String>(Data::PWD)?;
        // SAFETY: We trust libghostty to return a valid borrowed string,
        // while we uphold that no mutation could happen during its lifetime.
        let str = unsafe { str.to_bytes() };
        std::str::from_utf8(str).map_err(|_| Error::InvalidValue)
    }
    /// The total number of rows in the active screen including scrollback.
    pub fn total_rows(&self) -> Result<usize> {
        self.get(Data::TOTAL_ROWS)
    }
    /// The number of scrollback rows (total rows minus viewport rows).
    pub fn scrollback_rows(&self) -> Result<usize> {
        self.get(Data::SCROLLBACK_ROWS)
    }

    /// The effective foreground color (override or default).
    pub fn fg_color(&self) -> Result<Option<RgbColor>> {
        self.get_optional::<ffi::ColorRgb>(Data::COLOR_FOREGROUND)
            .map(|v| v.map(Into::into))
    }
    /// The default foreground color (ignoring any OSC override).
    pub fn default_fg_color(&self) -> Result<Option<RgbColor>> {
        self.get_optional::<ffi::ColorRgb>(Data::COLOR_FOREGROUND_DEFAULT)
            .map(|v| v.map(Into::into))
    }
    /// Set the default foreground color.
    pub fn set_default_fg_color(&mut self, v: Option<RgbColor>) -> Result<&mut Self> {
        self.set_optional(Opt::COLOR_FOREGROUND, v.map(ffi::ColorRgb::from).as_ref())?;
        Ok(self)
    }

    /// The effective background color (override or default).
    pub fn bg_color(&self) -> Result<Option<RgbColor>> {
        self.get_optional::<ffi::ColorRgb>(Data::COLOR_BACKGROUND)
            .map(|v| v.map(Into::into))
    }
    /// The default background color (ignoring any OSC override).
    pub fn default_bg_color(&self) -> Result<Option<RgbColor>> {
        self.get_optional::<ffi::ColorRgb>(Data::COLOR_BACKGROUND_DEFAULT)
            .map(|v| v.map(Into::into))
    }
    /// Set the default background color.
    pub fn set_default_bg_color(&mut self, v: Option<RgbColor>) -> Result<&mut Self> {
        self.set_optional(Opt::COLOR_BACKGROUND, v.map(ffi::ColorRgb::from).as_ref())?;
        Ok(self)
    }

    /// The effective cursor color (override or default).
    pub fn cursor_color(&self) -> Result<Option<RgbColor>> {
        self.get_optional::<ffi::ColorRgb>(Data::COLOR_CURSOR)
            .map(|v| v.map(Into::into))
    }
    /// The default cursor color (ignoring any OSC override).
    pub fn default_cursor_color(&self) -> Result<Option<RgbColor>> {
        self.get_optional::<ffi::ColorRgb>(Data::COLOR_CURSOR_DEFAULT)
            .map(|v| v.map(Into::into))
    }
    /// Set the default cursor color.
    pub fn set_default_cursor_color(&mut self, v: Option<RgbColor>) -> Result<&mut Self> {
        self.set_optional(Opt::COLOR_CURSOR, v.map(ffi::ColorRgb::from).as_ref())?;
        Ok(self)
    }

    /// Set the default cursor style used by DECSCUSR reset (CSI 0 q).
    ///
    /// Passing `None` resets to libghostty's built-in block cursor default.
    pub fn set_default_cursor_style(&mut self, v: Option<CursorStyle>) -> Result<&mut Self> {
        let raw = v.map(|v| v as ffi::TerminalCursorStyle::Type);
        self.set_optional(Opt::DEFAULT_CURSOR_STYLE, raw.as_ref())?;
        Ok(self)
    }

    /// Set whether the default cursor blinks when reset by DECSCUSR (CSI 0 q).
    ///
    /// Passing `None` resets to libghostty's built-in non-blinking default.
    pub fn set_default_cursor_blink(&mut self, v: Option<bool>) -> Result<&mut Self> {
        self.set_optional(Opt::DEFAULT_CURSOR_BLINK, v.as_ref())?;
        Ok(self)
    }

    /// The current 256-color palette.
    pub fn color_palette(&self) -> Result<Palette> {
        self.get::<RawPalette>(Data::COLOR_PALETTE)
            .map(Palette::from)
    }
    /// The default 256-color palette (ignoring any OSC overrides).
    pub fn default_color_palette(&self) -> Result<Palette> {
        self.get::<RawPalette>(Data::COLOR_PALETTE_DEFAULT)
            .map(Palette::from)
    }
    /// Set the default 256-color palette.
    pub fn set_default_color_palette(&mut self, v: Option<Palette>) -> Result<&mut Self> {
        self.set_optional::<RawPalette>(Opt::COLOR_PALETTE, v.map(Into::into).as_ref())?;
        Ok(self)
    }

    /// Set the maximum bytes the APC handler will buffer for all protocols.
    ///
    /// This prevents malicious input from causing unbounded memory allocation.
    /// A `None` value removes all overrides, reverting to the built-in defaults.
    pub fn set_apc_max_bytes(&mut self, max: Option<usize>) -> Result<&mut Self> {
        self.set_optional(Opt::APC_MAX_BYTES, max.as_ref())?;
        Ok(self)
    }

    /// Enable or disable Glyph Protocol APC handling.
    ///
    /// Disabling the protocol makes the terminal ignore Glyph Protocol APC
    /// sequences and clears the session's glyph glossary.
    pub fn set_glyph_protocol_enabled(&mut self, enabled: bool) -> Result<&mut Self> {
        self.set(Opt::GLYPH_PROTOCOL, &enabled)?;
        Ok(self)
    }

    /// Set the maximum scrollback allocation in bytes.
    ///
    /// This is an estimate: libghostty prunes at page granularity (about
    /// 400 KiB). It works alongside [the line limit](Self::set_scrollback_max_lines);
    /// the first limit reached wins. Lowering the limit immediately removes
    /// eligible complete pages. Zero disables scrollback and erases retained
    /// history. `None` removes the byte limit.
    pub fn set_scrollback_max_bytes(&mut self, max: Option<usize>) -> Result<&mut Self> {
        self.set_optional(Opt::SCROLLBACK_MAX_BYTES, max.as_ref())?;
        Ok(self)
    }
    /// The configured maximum scrollback allocation in bytes, or `None`
    /// when unlimited.
    pub fn scrollback_max_bytes(&self) -> Result<Option<usize>> {
        self.get_optional(Data::SCROLLBACK_MAX_BYTES)
    }
    /// Set the maximum number of physical lines retained in scrollback.
    ///
    /// This is an estimate: libghostty prunes at page granularity, so the
    /// retained line count is almost always somewhat higher. `None` removes
    /// the line limit.
    pub fn set_scrollback_max_lines(&mut self, max: Option<usize>) -> Result<&mut Self> {
        self.set_optional(Opt::SCROLLBACK_MAX_LINES, max.as_ref())?;
        Ok(self)
    }
    /// The configured maximum number of scrollback lines, or `None` when
    /// unlimited.
    pub fn scrollback_max_lines(&self) -> Result<Option<usize>> {
        self.get_optional(Data::SCROLLBACK_MAX_LINES)
    }

    /// Set the maximum decoded bytes a single Kitty clipboard protocol
    /// (OSC 5522) write transaction may accumulate.
    ///
    /// Data beyond the limit fails the whole transaction. `None` reverts to
    /// the built-in default of 64 MiB. This does not apply to OSC 52 writes.
    pub fn set_clipboard_write_max_bytes(&mut self, max: Option<usize>) -> Result<&mut Self> {
        self.set_optional(Opt::CLIPBOARD_WRITE_MAX_BYTES, max.as_ref())?;
        Ok(self)
    }
    /// The configured maximum decoded bytes per Kitty clipboard write.
    pub fn clipboard_write_max_bytes(&self) -> Result<usize> {
        self.get(Data::CLIPBOARD_WRITE_MAX_BYTES)
    }

    /// Set the maximum number of replay-safe VT continuation bytes retained.
    ///
    /// Continuation bytes reconstruct an escape sequence or UTF-8 codepoint
    /// which was unfinished at the end of the most recent write. They are
    /// used by [snapshots](crate::snapshot) and may be exported with
    /// [`Terminal::continuation`]. Tracking is off by default; zero disables
    /// it. Enabling tracking while the parser is already mid-sequence makes
    /// the current continuation unavailable until the next ground.
    pub fn set_continuation_max_bytes(&mut self, max: usize) -> Result<&mut Self> {
        self.set(Opt::CONTINUATION_MAX_BYTES, &max)?;
        Ok(self)
    }
    /// The configured maximum retained continuation size. Zero means
    /// tracking is disabled.
    pub fn continuation_max_bytes(&self) -> Result<usize> {
        self.get(Data::CONTINUATION_MAX_BYTES)
    }
    /// Copy the replay-safe VT continuation.
    ///
    /// The continuation is the exact byte suffix needed to reconstruct
    /// unfinished parser or UTF-8 decoder state in an equivalent terminal. It
    /// is empty when the stream is at ground. Returns [`Error::InvalidValue`]
    /// when tracking is disabled or the current continuation is unavailable.
    pub fn continuation(&self) -> Result<Vec<u8>> {
        let mut required = 0usize;
        let result = unsafe {
            ffi::ghostty_terminal_continuation_buf(
                self.inner.as_ptr(),
                std::ptr::null_mut(),
                0,
                &raw mut required,
            )
        };
        match from_result(result) {
            Err(Error::OutOfSpace { .. }) => {}
            Err(e) => return Err(e),
            Ok(()) => return Ok(Vec::new()),
        }
        if required == 0 {
            return Ok(Vec::new());
        }
        let mut buf = vec![0u8; required];
        let mut written = 0usize;
        let result = unsafe {
            ffi::ghostty_terminal_continuation_buf(
                self.inner.as_ptr(),
                buf.as_mut_ptr(),
                buf.len(),
                &raw mut written,
            )
        };
        from_result_with_len(result, written)?;
        buf.truncate(written);
        Ok(buf)
    }
    /// Stream the replay-safe VT continuation to a writer.
    ///
    /// See [`Terminal::continuation`].
    pub fn continuation_write(&self, writer: &mut dyn Write) -> Result<()> {
        crate::io::with_writer(writer, |w| unsafe {
            ffi::ghostty_terminal_continuation_write(self.inner.as_ptr(), w)
        })
    }

    /// Enable window title reports in response to CSI 21 t.
    ///
    /// This is off by default because a running program could set a title
    /// and query it back into the input stream.
    pub fn set_title_report(&mut self, enabled: bool) -> Result<&mut Self> {
        self.set(Opt::TITLE_REPORT, &enabled)?;
        Ok(self)
    }

    /// Set the name of the terminfo entry this terminal runs as, reported in
    /// response to an XTGETTCAP query for `TN`.
    ///
    /// `None` clears the name, and nothing is reported. Names longer than
    /// 128 bytes return [`Error::InvalidValue`].
    pub fn set_terminfo_name(&mut self, name: Option<&str>) -> Result<&mut Self> {
        let raw = name.map(ffi::String::from);
        self.set_optional(Opt::TERMINFO_NAME, raw.as_ref())?;
        Ok(self)
    }

    /// Set whether a resize may pull rows out of scrollback back into the
    /// active area.
    ///
    /// Set this to false when the pty keeps its own screen buffer without
    /// scrollback, such as Windows ConPTY. The default is true.
    pub fn set_resize_pull_scrollback(&mut self, enabled: bool) -> Result<&mut Self> {
        self.set(Opt::RESIZE_PULL_SCROLLBACK, &enabled)?;
        Ok(self)
    }

    // --- Query answers --- //

    /// Set the answer to device attributes queries (CSI c, CSI > c, CSI = c).
    ///
    /// `None` lets libghostty answer with its built-in attributes. The
    /// default is Ghostty's own answer.
    pub fn set_device_attributes(&mut self, attrs: Option<DeviceAttributes>) {
        self.shared().device_attributes = attrs.map(Into::into);
    }

    /// Set the version string reported to XTVERSION queries (CSI > q),
    /// such as `"myterm 1.0"`. `None` reports libghostty's own version.
    pub fn set_xtversion(&mut self, version: Option<String>) {
        self.shared().xtversion = version;
    }

    /// Set the bytes written to the pty in response to ENQ (0x05).
    /// `None` sends no response.
    pub fn set_enquiry_response(&mut self, response: Option<Vec<u8>>) {
        self.shared().enquiry = response;
    }

    /// Set the color scheme reported to CSI ? 996 n queries.
    /// `None` ignores the query.
    pub fn set_color_scheme(&mut self, scheme: Option<ColorScheme>) {
        self.shared().color_scheme = scheme;
    }

    /// Set whether size queries (CSI 14/16/18 t) and mode 2048 reports are
    /// answered from the geometry given to [`Terminal::resize`]. Nothing is
    /// reported before the first resize. The default is on.
    pub fn set_size_reports(&mut self, enabled: bool) {
        self.shared().size_reports = enabled;
    }

    /// Set the policy for clipboard writes by the running program (OSC 52,
    /// OSC 1337, OSC 5522). The default is [`ClipboardWritePolicy::Allow`].
    pub fn set_clipboard_write_policy(&mut self, policy: ClipboardWritePolicy) {
        self.shared().clipboard_write_policy = policy;
    }

    /// Set the render hold callback.
    ///
    /// The callback runs synchronously inside [`Terminal::vt_write`] with
    /// `held` set to true when the running program starts synchronized
    /// output (mode 2026) and false when the hold ends. When a hold begins,
    /// the terminal contains exactly the frame the program wants left on
    /// screen: capture it by updating a render state from the given terminal,
    /// then stop updating until the hold ends. [`Event::RenderHold`] is
    /// queued as well.
    ///
    /// The terminal has no clock and never ends a hold on its own. Time the
    /// hold out (Ghostty uses one second) by resetting
    /// [`Mode::SYNC_OUTPUT`] with [`Terminal::set_mode`].
    ///
    /// The callback must not write to the terminal or drain its events.
    pub fn set_render_hold(&mut self, callback: Option<RenderHoldFn>) {
        self.shared().render_hold = callback;
    }

    fn install_callbacks(&self) -> Result<()> {
        macro_rules! install {
            ($opt:ident, $ty:ty, $cb:expr) => {{
                let f: $ty = Some($cb);
                self.set_ptr(
                    Opt::$opt,
                    f.map_or(std::ptr::null(), |p| p as *const c_void),
                )?;
            }};
        }
        install!(WRITE_PTY, ffi::TerminalWritePtyFn, cb_write_pty);
        install!(BELL, ffi::TerminalBellFn, cb_bell);
        install!(TITLE_CHANGED, ffi::TerminalTitleChangedFn, cb_title_changed);
        install!(PWD_CHANGED, ffi::TerminalPwdChangedFn, cb_pwd_changed);
        install!(ENQUIRY, ffi::TerminalEnquiryFn, cb_enquiry);
        install!(XTVERSION, ffi::TerminalXtversionFn, cb_xtversion);
        install!(SIZE, ffi::TerminalSizeFn, cb_size);
        install!(COLOR_SCHEME, ffi::TerminalColorSchemeFn, cb_color_scheme);
        install!(
            DEVICE_ATTRIBUTES,
            ffi::TerminalDeviceAttributesFn,
            cb_device_attributes
        );
        install!(
            CLIPBOARD_WRITE,
            ffi::TerminalClipboardWriteFn,
            cb_clipboard_write
        );
        install!(
            DESKTOP_NOTIFICATION,
            ffi::TerminalDesktopNotificationFn,
            cb_desktop_notification
        );
        install!(
            PROGRESS_REPORT,
            ffi::TerminalProgressReportFn,
            cb_progress_report
        );
        install!(RENDER_HOLD, ffi::TerminalRenderHoldFn, cb_render_hold);
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe {
            ffi::ghostty_terminal_free(self.inner.as_ptr());
            // SAFETY: `shared` was leaked from a Box in `from_raw` and nothing
            // references it once the terminal is freed.
            drop(Box::from_raw(self.shared.as_ptr()));
        }
    }
}

//---------------------------------------
// Callbacks
//---------------------------------------

/// # Safety
///
/// `ud` must be the userdata pointer installed by `Terminal::from_raw`.
unsafe fn shared_from_userdata<'a>(ud: *mut c_void) -> &'a mut Shared {
    unsafe { &mut *ud.cast::<Shared>() }
}

unsafe fn string_from_terminal(t: ffi::Terminal, tag: ffi::TerminalData::Type) -> String {
    let mut value = MaybeUninit::<ffi::String>::zeroed();
    let result = unsafe { ffi::ghostty_terminal_get(t, tag, value.as_mut_ptr().cast()) };
    if result != ffi::Result::SUCCESS {
        return String::new();
    }
    // SAFETY: The string is valid for the duration of the callback.
    let bytes = unsafe { value.assume_init().to_bytes() };
    String::from_utf8_lossy(bytes).into_owned()
}

unsafe extern "C" fn cb_write_pty(_t: ffi::Terminal, ud: *mut c_void, data: *const u8, len: usize) {
    let shared = unsafe { shared_from_userdata(ud) };
    let data = if data.is_null() || len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
    };
    shared.events.push(Event::PtyWrite(data));
}

unsafe extern "C" fn cb_bell(_t: ffi::Terminal, ud: *mut c_void) {
    let shared = unsafe { shared_from_userdata(ud) };
    shared.events.push(Event::Bell);
}

unsafe extern "C" fn cb_title_changed(t: ffi::Terminal, ud: *mut c_void) {
    let shared = unsafe { shared_from_userdata(ud) };
    let title = unsafe { string_from_terminal(t, Data::TITLE) };
    shared.events.push(Event::TitleChanged(title));
}

unsafe extern "C" fn cb_pwd_changed(t: ffi::Terminal, ud: *mut c_void) {
    let shared = unsafe { shared_from_userdata(ud) };
    let pwd = unsafe { string_from_terminal(t, Data::PWD) };
    shared.events.push(Event::PwdChanged(pwd));
}

unsafe extern "C" fn cb_enquiry(_t: ffi::Terminal, ud: *mut c_void) -> ffi::String {
    let shared = unsafe { shared_from_userdata(ud) };
    match &shared.enquiry {
        Some(bytes) => ffi::String {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        },
        None => ffi::String {
            ptr: std::ptr::null(),
            len: 0,
        },
    }
}

unsafe extern "C" fn cb_xtversion(_t: ffi::Terminal, ud: *mut c_void) -> ffi::String {
    let shared = unsafe { shared_from_userdata(ud) };
    match &shared.xtversion {
        Some(version) => ffi::String::from(version.as_str()),
        None => ffi::String {
            ptr: std::ptr::null(),
            len: 0,
        },
    }
}

unsafe extern "C" fn cb_size(
    _t: ffi::Terminal,
    ud: *mut c_void,
    out: *mut ffi::SizeReportSize,
) -> bool {
    let shared = unsafe { shared_from_userdata(ud) };
    match shared.size {
        Some(size) if shared.size_reports => {
            unsafe { *out = size };
            true
        }
        _ => false,
    }
}

unsafe extern "C" fn cb_color_scheme(
    _t: ffi::Terminal,
    ud: *mut c_void,
    out: *mut ffi::ColorScheme::Type,
) -> bool {
    let shared = unsafe { shared_from_userdata(ud) };
    match shared.color_scheme {
        Some(scheme) => {
            unsafe { *out = scheme.into() };
            true
        }
        None => false,
    }
}

unsafe extern "C" fn cb_device_attributes(
    _t: ffi::Terminal,
    ud: *mut c_void,
    out: *mut ffi::DeviceAttributes,
) -> bool {
    let shared = unsafe { shared_from_userdata(ud) };
    match shared.device_attributes {
        Some(attrs) => {
            unsafe { *out = attrs };
            true
        }
        None => false,
    }
}

unsafe extern "C" fn cb_clipboard_write(
    _t: ffi::Terminal,
    ud: *mut c_void,
    write: *const ffi::ClipboardWrite,
) {
    let shared = unsafe { shared_from_userdata(ud) };
    // SAFETY: libghostty guarantees a valid request for the callback duration.
    let raw = unsafe { &*write };
    let result = match shared.clipboard_write_policy {
        ClipboardWritePolicy::Allow => {
            let contents: &[ffi::ClipboardContent] = if raw.contents.is_null() {
                &[]
            } else {
                unsafe { std::slice::from_raw_parts(raw.contents, raw.contents_len) }
            };
            let contents = contents
                .iter()
                .map(|content| unsafe {
                    ClipboardContent {
                        mime: String::from_utf8_lossy(content.mime.to_bytes()).into_owned(),
                        data: content.data.to_bytes().to_vec(),
                    }
                })
                .collect();
            shared.events.push(Event::ClipboardWrite(ClipboardWrite {
                location: raw
                    .location
                    .try_into()
                    .unwrap_or(ClipboardLocation::Standard),
                contents,
                name: String::from_utf8_lossy(unsafe { raw.name.to_bytes() }).into_owned(),
                granted: raw.granted,
                can_remember: raw.can_remember,
            }));
            ffi::ClipboardWriteResult::SUCCESS
        }
        ClipboardWritePolicy::Deny => ffi::ClipboardWriteResult::DENIED,
    };
    let reply = ffi::ClipboardWriteReply {
        size: std::mem::size_of::<ffi::ClipboardWriteReply>(),
        result,
        remember: false,
    };
    if let Some(reply_fn) = raw.reply {
        unsafe { reply_fn(write, &raw const reply) };
    }
}

unsafe extern "C" fn cb_desktop_notification(
    _t: ffi::Terminal,
    ud: *mut c_void,
    notification: *const ffi::TerminalDesktopNotification,
) {
    let shared = unsafe { shared_from_userdata(ud) };
    let raw = unsafe { &*notification };
    shared.events.push(Event::DesktopNotification {
        title: String::from_utf8_lossy(unsafe { raw.title.to_bytes() }).into_owned(),
        body: String::from_utf8_lossy(unsafe { raw.body.to_bytes() }).into_owned(),
    });
}

unsafe extern "C" fn cb_progress_report(
    _t: ffi::Terminal,
    ud: *mut c_void,
    report: *const ffi::TerminalProgressReport,
) {
    let shared = unsafe { shared_from_userdata(ud) };
    let raw = unsafe { &*report };
    shared.events.push(Event::ProgressReport {
        state: raw.state.try_into().unwrap_or(ProgressState::Remove),
        progress: u8::try_from(raw.progress).ok(),
    });
}

unsafe extern "C" fn cb_render_hold(t: ffi::Terminal, ud: *mut c_void, held: bool) {
    let shared = unsafe { shared_from_userdata(ud) };
    shared.events.push(Event::RenderHold(held));
    let Some(mut callback) = shared.render_hold.take() else {
        return;
    };
    {
        // A borrowed view of the terminal for the callback. It shares the
        // heap state with the owning handle, but `&Terminal` only exposes
        // methods that read the C terminal, so nothing aliases `shared`
        // while the closure runs (it was taken out above).
        let view = ManuallyDrop::new(Terminal {
            inner: unsafe { NonNull::new_unchecked(t) },
            shared: unsafe { NonNull::new_unchecked(ud.cast::<Shared>()) },
        });
        callback(&view, held);
    }
    let shared = unsafe { shared_from_userdata(ud) };
    if shared.render_hold.is_none() {
        shared.render_hold = Some(callback);
    }
}

//---------------------------------------
// Events
//---------------------------------------

/// A side effect or query response produced while processing VT input.
///
/// Drain events with [`Terminal::events`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Bytes the terminal wants written back to the pty, such as query
    /// responses and mode reports.
    PtyWrite(Vec<u8>),
    /// A BEL character (0x07) was received.
    Bell,
    /// The title changed via OSC 0 or OSC 2. Carries the new title.
    TitleChanged(String),
    /// The working directory changed via OSC 7, OSC 9 or OSC 1337. Carries
    /// the raw value the shell emitted (a `file://` URI for OSC 7).
    PwdChanged(String),
    /// The running program wrote to the clipboard and the
    /// [policy](Terminal::set_clipboard_write_policy) allowed it.
    ClipboardWrite(ClipboardWrite),
    /// The running program requested a desktop notification (OSC 9, OSC 777).
    DesktopNotification {
        /// Notification title, empty when the protocol omits it.
        title: String,
        /// Notification body.
        body: String,
    },
    /// The running program reported progress (OSC 9;4).
    ProgressReport {
        /// The progress state.
        state: ProgressState,
        /// Progress from 0 through 100, or `None` when omitted.
        progress: Option<u8>,
    },
    /// Synchronized output (mode 2026) began (`true`) or ended (`false`).
    /// See [`Terminal::set_render_hold`].
    RenderHold(bool),
}

/// Policy for clipboard writes by the running program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipboardWritePolicy {
    /// Accept the write and queue [`Event::ClipboardWrite`].
    #[default]
    Allow,
    /// Refuse the write. Protocols with an acknowledgement (OSC 5522) are
    /// answered with a permission error.
    Deny,
}

/// A semantic, atomic clipboard write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardWrite {
    /// The clipboard's destination.
    pub location: ClipboardLocation,
    /// MIME representations of one logical value, to be committed
    /// atomically. Empty requests that the destination be cleared.
    pub contents: Vec<ClipboardContent>,
    /// Name of the writing program for permission prompts, if the protocol
    /// carries one (OSC 5522). Empty otherwise.
    pub name: String,
    /// Whether the terminal already holds a session grant for this program.
    pub granted: bool,
    /// Whether the program supplied a session password, so a decision could
    /// be remembered.
    pub can_remember: bool,
}

/// One MIME representation in a clipboard write.
///
/// The data is binary-safe and has already been decoded from any
/// protocol-level encoding. A zero-length data string is an explicit empty
/// representation; it does not clear the clipboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardContent {
    /// MIME type of the representation.
    pub mime: String,
    /// Decoded representation data.
    pub data: Vec<u8>,
}

/// Clipboard destination for a clipboard write.
///
/// Protocol-specific destination identifiers are normalized to these values
/// before the clipboard write event is queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq, int_enum::IntEnum)]
#[repr(i32)]
pub enum ClipboardLocation {
    /// The standard system clipboard.
    Standard = ffi::ClipboardLocation::STANDARD,
    /// The selection clipboard.
    Selection = ffi::ClipboardLocation::SELECTION,
    /// The primary selection clipboard.
    Primary = ffi::ClipboardLocation::PRIMARY,
}

/// State of a terminal progress report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, int_enum::IntEnum)]
#[repr(i32)]
pub enum ProgressState {
    /// Remove any visible progress indication.
    Remove = ffi::TerminalProgressState::REMOVE,
    /// Show determinate progress.
    Set = ffi::TerminalProgressState::SET,
    /// Show a failed progress state.
    Error = ffi::TerminalProgressState::ERROR,
    /// Show indeterminate progress.
    Indeterminate = ffi::TerminalProgressState::INDETERMINATE,
    /// Show paused progress.
    Pause = ffi::TerminalProgressState::PAUSE,
}

//---------------------------------------
// Points and scrolling
//---------------------------------------

/// A point in the terminal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    /// Active area where the cursor can move.
    Active(PointCoordinate),
    /// Visible viewport (changes when scrolled).
    Viewport(PointCoordinate),
    /// Full screen including scrollback.
    Screen(PointCoordinate),
    /// Scrollback history only (before active area).
    History(PointCoordinate),
}

impl From<Point> for ffi::Point {
    fn from(value: Point) -> Self {
        let (tag, coord) = match value {
            Point::Active(coord) => (ffi::PointTag::ACTIVE, coord),
            Point::Viewport(coord) => (ffi::PointTag::VIEWPORT, coord),
            Point::Screen(coord) => (ffi::PointTag::SCREEN, coord),
            Point::History(coord) => (ffi::PointTag::HISTORY, coord),
        };
        Self {
            tag,
            value: ffi::PointValue {
                coordinate: coord.into(),
            },
        }
    }
}

/// A coordinate space for converting grid references back to points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointSpace {
    /// Active area where the cursor can move.
    Active,
    /// Visible viewport, which changes when scrolled.
    Viewport,
    /// Full screen including scrollback.
    Screen,
    /// Scrollback history only, before the active area.
    History,
}

impl PointSpace {
    pub(crate) fn into_raw(self) -> ffi::PointTag::Type {
        match self {
            Self::Active => ffi::PointTag::ACTIVE,
            Self::Viewport => ffi::PointTag::VIEWPORT,
            Self::Screen => ffi::PointTag::SCREEN,
            Self::History => ffi::PointTag::HISTORY,
        }
    }
}

/// A coordinate in the terminal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointCoordinate {
    /// Column (0-indexed).
    pub x: u16,
    /// Row (0-indexed). May exceed page size for screen/history tags.
    pub y: u32,
}
impl From<PointCoordinate> for ffi::PointCoordinate {
    fn from(value: PointCoordinate) -> Self {
        let PointCoordinate { x, y } = value;
        Self { x, y }
    }
}
impl From<ffi::PointCoordinate> for PointCoordinate {
    fn from(value: ffi::PointCoordinate) -> Self {
        let ffi::PointCoordinate { x, y } = value;
        Self { x, y }
    }
}

/// Scroll viewport behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollViewport {
    /// Scroll to the top of the scrollback.
    Top,
    /// Scroll to the bottom (active area).
    Bottom,
    /// Scroll by a delta amount (up is negative).
    Delta(isize),
    /// Scroll to an absolute row offset from the top of the scrollback.
    Row(usize),
    /// Scroll by prompts relative to the viewport top (negative moves backward).
    /// Requires shell semantic marks; zero or no matching prompt is a no-op.
    /// Clamps to the active area. A final continuation may snap to the active area.
    DeltaPrompt(isize),
}
impl From<ScrollViewport> for ffi::TerminalScrollViewport {
    fn from(value: ScrollViewport) -> Self {
        match value {
            ScrollViewport::Top => Self {
                tag: ffi::TerminalScrollViewportTag::TOP,
                value: ffi::TerminalScrollViewportValue::default(),
            },
            ScrollViewport::Bottom => Self {
                tag: ffi::TerminalScrollViewportTag::BOTTOM,
                value: ffi::TerminalScrollViewportValue::default(),
            },
            ScrollViewport::Delta(delta) => Self {
                tag: ffi::TerminalScrollViewportTag::DELTA,
                value: ffi::TerminalScrollViewportValue { delta },
            },
            ScrollViewport::Row(row) => Self {
                tag: ffi::TerminalScrollViewportTag::ROW,
                value: ffi::TerminalScrollViewportValue { row },
            },
            ScrollViewport::DeltaPrompt(delta_prompt) => Self {
                tag: ffi::TerminalScrollViewportTag::DELTA_PROMPT,
                value: ffi::TerminalScrollViewportValue { delta_prompt },
            },
        }
    }
}

//---------------------------------------
// Modes
//---------------------------------------

/// A terminal mode consisting of its value and its kind (DEC/ANSI).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Mode(pub ffi::Mode);

#[allow(missing_docs, reason = "no upstream documentation provided")]
impl Mode {
    const ANSI_BIT: u16 = 1 << 15;

    /// Create a new mode from its numeric value and its kind.
    #[must_use]
    pub const fn new(v: u16, kind: ModeKind) -> Self {
        match kind {
            ModeKind::Ansi => Self((v & 0x7fff) | Self::ANSI_BIT),
            ModeKind::Dec => Self(v & 0x7fff),
        }
    }

    /// The numeric value of the mode.
    #[must_use]
    pub const fn value(self) -> u16 {
        (self.0) & 0x7fff
    }

    /// The kind of the mode (DEC/ANSI).
    #[must_use]
    pub const fn kind(self) -> ModeKind {
        if (self.0) & Self::ANSI_BIT > 0 {
            ModeKind::Ansi
        } else {
            ModeKind::Dec
        }
    }

    pub const KAM: Self = Self::new(2, ModeKind::Ansi);
    pub const INSERT: Self = Self::new(4, ModeKind::Ansi);
    pub const SRM: Self = Self::new(12, ModeKind::Ansi);
    pub const LINEFEED: Self = Self::new(20, ModeKind::Ansi);

    pub const DECCKM: Self = Self::new(1, ModeKind::Dec);
    pub const _132_COLUMN: Self = Self::new(3, ModeKind::Dec);
    pub const SLOW_SCROLL: Self = Self::new(4, ModeKind::Dec);
    pub const REVERSE_COLORS: Self = Self::new(5, ModeKind::Dec);
    pub const ORIGIN: Self = Self::new(6, ModeKind::Dec);
    pub const WRAPAROUND: Self = Self::new(7, ModeKind::Dec);
    pub const AUTOREPEAT: Self = Self::new(8, ModeKind::Dec);
    pub const X10_MOUSE: Self = Self::new(9, ModeKind::Dec);
    pub const CURSOR_BLINKING: Self = Self::new(12, ModeKind::Dec);
    pub const CURSOR_VISIBLE: Self = Self::new(25, ModeKind::Dec);
    pub const ENABLE_MODE3: Self = Self::new(40, ModeKind::Dec);
    pub const REVERSE_WRAP: Self = Self::new(45, ModeKind::Dec);
    pub const ALT_SCREEN_LEGACY: Self = Self::new(47, ModeKind::Dec);
    pub const KEYPAD_KEYS: Self = Self::new(66, ModeKind::Dec);
    pub const BACKARROW_KEY_MODE: Self = Self::new(67, ModeKind::Dec);
    pub const LEFT_RIGHT_MARGIN: Self = Self::new(69, ModeKind::Dec);
    pub const NORMAL_MOUSE: Self = Self::new(1000, ModeKind::Dec);
    pub const BUTTON_MOUSE: Self = Self::new(1002, ModeKind::Dec);
    pub const ANY_MOUSE: Self = Self::new(1003, ModeKind::Dec);
    pub const FOCUS_EVENT: Self = Self::new(1004, ModeKind::Dec);
    pub const UTF8_MOUSE: Self = Self::new(1005, ModeKind::Dec);
    pub const SGR_MOUSE: Self = Self::new(1006, ModeKind::Dec);
    pub const ALT_SCROLL: Self = Self::new(1007, ModeKind::Dec);
    pub const URXVT_MOUSE: Self = Self::new(1015, ModeKind::Dec);
    pub const SGR_PIXELS_MOUSE: Self = Self::new(1016, ModeKind::Dec);
    pub const NUMLOCK_KEYPAD: Self = Self::new(1035, ModeKind::Dec);
    pub const ALT_ESC_PREFIX: Self = Self::new(1036, ModeKind::Dec);
    pub const ALT_SENDS_ESC: Self = Self::new(1039, ModeKind::Dec);
    pub const REVERSE_WRAP_EXT: Self = Self::new(1045, ModeKind::Dec);
    pub const ALT_SCREEN: Self = Self::new(1047, ModeKind::Dec);
    pub const SAVE_CURSOR: Self = Self::new(1048, ModeKind::Dec);
    pub const ALT_SCREEN_SAVE: Self = Self::new(1049, ModeKind::Dec);
    pub const BRACKETED_PASTE: Self = Self::new(2004, ModeKind::Dec);
    pub const SYNC_OUTPUT: Self = Self::new(2026, ModeKind::Dec);
    pub const GRAPHEME_CLUSTER: Self = Self::new(2027, ModeKind::Dec);
    pub const COLOR_SCHEME_REPORT: Self = Self::new(2031, ModeKind::Dec);
    pub const VISIBILITY_REPORT: Self = Self::new(2033, ModeKind::Dec);
    pub const IN_BAND_RESIZE: Self = Self::new(2048, ModeKind::Dec);
    pub const PASTE_EVENTS: Self = Self::new(5522, ModeKind::Dec);
}

/// The kind of a terminal mode.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModeKind {
    /// DEC terminal mode.
    Dec,
    /// ANSI terminal mode.
    Ansi,
}

impl From<Mode> for ffi::Mode {
    fn from(value: Mode) -> Self {
        value.0
    }
}

//---------------------------------------
// Device attributes
//---------------------------------------

/// Device attributes response data for all three DA levels.
///
/// Set with [`Terminal::set_device_attributes`] to answer CSI c, CSI > c
/// and CSI = c queries. The terminal uses whichever sub-struct matches the
/// request type.
#[derive(Debug, Clone, Copy)]
pub struct DeviceAttributes {
    /// Primary device attributes (DA1).
    pub primary: PrimaryDeviceAttributes,
    /// Secondary device attributes (DA2).
    pub secondary: SecondaryDeviceAttributes,
    /// Tertiary device attributes (DA3).
    pub tertiary: TertiaryDeviceAttributes,
}

impl Default for DeviceAttributes {
    /// Ghostty's own device attributes: a level 2 terminal with ANSI color
    /// (`CSI ?62;22c`), a VT220 with firmware 10 (`CSI >1;10;0c`), and
    /// unit id 0.
    fn default() -> Self {
        Self {
            primary: PrimaryDeviceAttributes::new(
                ConformanceLevel::LEVEL_2,
                &[DeviceAttributeFeature::ANSI_COLOR],
            ),
            secondary: SecondaryDeviceAttributes {
                device_type: DeviceType::VT220,
                firmware_version: 10,
                rom_cartridge: 0,
            },
            tertiary: TertiaryDeviceAttributes { unit_id: 0 },
        }
    }
}

impl From<DeviceAttributes> for ffi::DeviceAttributes {
    fn from(value: DeviceAttributes) -> Self {
        Self {
            primary: value.primary.into(),
            secondary: value.secondary.into(),
            tertiary: value.tertiary.into(),
        }
    }
}

/// Primary device attributes (DA1) response data.
#[derive(Debug, Clone, Copy)]
pub struct PrimaryDeviceAttributes(ffi::DeviceAttributesPrimary);

impl PrimaryDeviceAttributes {
    /// Construct primary device attributes from a conformance level
    /// and an array of device attribute features.
    ///
    /// # Panics
    ///
    /// **Panics** when more than 64 features are given.
    #[must_use]
    pub const fn new(
        conformance_level: ConformanceLevel,
        features: &[DeviceAttributeFeature],
    ) -> Self {
        assert!(features.len() <= 64);

        let mut f = [0u16; 64];
        let mut i = 0;
        while i < features.len() {
            f[i] = features[i].0;
            i += 1;
        }

        Self(ffi::DeviceAttributesPrimary {
            conformance_level: conformance_level.0,
            features: f,
            num_features: features.len(),
        })
    }
}

impl From<PrimaryDeviceAttributes> for ffi::DeviceAttributesPrimary {
    fn from(value: PrimaryDeviceAttributes) -> Self {
        value.0
    }
}

/// The level of conformance to the behavior of a specific or a family of
/// physical terminal models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConformanceLevel(pub u16);

#[allow(missing_docs, reason = "self-explanatory")]
impl ConformanceLevel {
    pub const VT100: Self = Self(ffi::DA_CONFORMANCE_VT100);
    pub const VT101: Self = Self(ffi::DA_CONFORMANCE_VT101);
    pub const VT102: Self = Self(ffi::DA_CONFORMANCE_VT102);
    pub const VT125: Self = Self(ffi::DA_CONFORMANCE_VT125);
    pub const VT131: Self = Self(ffi::DA_CONFORMANCE_VT131);
    pub const VT132: Self = Self(ffi::DA_CONFORMANCE_VT132);
    pub const VT220: Self = Self(ffi::DA_CONFORMANCE_VT220);
    pub const VT240: Self = Self(ffi::DA_CONFORMANCE_VT240);
    pub const VT320: Self = Self(ffi::DA_CONFORMANCE_VT320);
    pub const VT340: Self = Self(ffi::DA_CONFORMANCE_VT340);
    pub const VT420: Self = Self(ffi::DA_CONFORMANCE_VT420);
    pub const VT510: Self = Self(ffi::DA_CONFORMANCE_VT510);
    pub const VT520: Self = Self(ffi::DA_CONFORMANCE_VT520);
    pub const VT525: Self = Self(ffi::DA_CONFORMANCE_VT525);
    /// Equivalent to a VT2xx terminal.
    pub const LEVEL_2: Self = Self(ffi::DA_CONFORMANCE_LEVEL_2);
    /// Equivalent to a VT3xx terminal.
    pub const LEVEL_3: Self = Self(ffi::DA_CONFORMANCE_LEVEL_3);
    /// Equivalent to a VT4xx terminal.
    pub const LEVEL_4: Self = Self(ffi::DA_CONFORMANCE_LEVEL_4);
    /// Equivalent to a VT5xx terminal.
    pub const LEVEL_5: Self = Self(ffi::DA_CONFORMANCE_LEVEL_5);
}

/// A feature that a terminal can report to support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceAttributeFeature(pub u16);

#[allow(missing_docs, reason = "no upstream documentation provided")]
impl DeviceAttributeFeature {
    pub const COLUMNS_132: Self = Self(ffi::DA_FEATURE_COLUMNS_132);
    pub const PRINTER: Self = Self(ffi::DA_FEATURE_PRINTER);
    pub const REGIS: Self = Self(ffi::DA_FEATURE_REGIS);
    pub const SIXEL: Self = Self(ffi::DA_FEATURE_SIXEL);
    pub const SELECTIVE_ERASE: Self = Self(ffi::DA_FEATURE_SELECTIVE_ERASE);
    pub const USER_DEFINED_KEYS: Self = Self(ffi::DA_FEATURE_USER_DEFINED_KEYS);
    pub const NATIONAL_REPLACEMENT: Self = Self(ffi::DA_FEATURE_NATIONAL_REPLACEMENT);
    pub const TECHNICAL_CHARACTERS: Self = Self(ffi::DA_FEATURE_TECHNICAL_CHARACTERS);
    pub const LOCATOR: Self = Self(ffi::DA_FEATURE_LOCATOR);
    pub const TERMINAL_STATE: Self = Self(ffi::DA_FEATURE_TERMINAL_STATE);
    pub const WINDOWING: Self = Self(ffi::DA_FEATURE_WINDOWING);
    pub const HORIZONTAL_SCROLLING: Self = Self(ffi::DA_FEATURE_HORIZONTAL_SCROLLING);
    pub const ANSI_COLOR: Self = Self(ffi::DA_FEATURE_ANSI_COLOR);
    pub const RECTANGULAR_EDITING: Self = Self(ffi::DA_FEATURE_RECTANGULAR_EDITING);
    pub const ANSI_TEXT_LOCATOR: Self = Self(ffi::DA_FEATURE_ANSI_TEXT_LOCATOR);
    pub const CLIPBOARD: Self = Self(ffi::DA_FEATURE_CLIPBOARD);
}

/// Secondary device attributes (DA2) response data.
///
/// Response format: CSI > Pp ; Pv ; Pc c
#[derive(Debug, Copy, Clone)]
pub struct SecondaryDeviceAttributes {
    /// Terminal type identifier (Pp).
    pub device_type: DeviceType,
    /// Firmware/patch version number (Pv).
    pub firmware_version: u16,
    /// ROM cartridge registration number (Pc). Always 0 for emulators.
    pub rom_cartridge: u16,
}

impl From<SecondaryDeviceAttributes> for ffi::DeviceAttributesSecondary {
    fn from(value: SecondaryDeviceAttributes) -> Self {
        Self {
            device_type: value.device_type.0,
            firmware_version: value.firmware_version,
            rom_cartridge: value.rom_cartridge,
        }
    }
}

/// The type of terminal device being emulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceType(pub u16);

#[allow(missing_docs, reason = "self-explanatory")]
impl DeviceType {
    pub const VT100: Self = Self(ffi::DA_DEVICE_TYPE_VT100);
    pub const VT220: Self = Self(ffi::DA_DEVICE_TYPE_VT220);
    pub const VT240: Self = Self(ffi::DA_DEVICE_TYPE_VT240);
    pub const VT330: Self = Self(ffi::DA_DEVICE_TYPE_VT330);
    pub const VT340: Self = Self(ffi::DA_DEVICE_TYPE_VT340);
    pub const VT320: Self = Self(ffi::DA_DEVICE_TYPE_VT320);
    pub const VT382: Self = Self(ffi::DA_DEVICE_TYPE_VT382);
    pub const VT420: Self = Self(ffi::DA_DEVICE_TYPE_VT420);
    pub const VT510: Self = Self(ffi::DA_DEVICE_TYPE_VT510);
    pub const VT520: Self = Self(ffi::DA_DEVICE_TYPE_VT520);
    pub const VT525: Self = Self(ffi::DA_DEVICE_TYPE_VT525);
}

/// Tertiary device attributes (DA3) response data.
///
/// Response format: DCS ! | D...D ST (DECRPTUI).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TertiaryDeviceAttributes {
    /// Unit ID encoded as 8 uppercase hex digits in the response.
    pub unit_id: u32,
}

impl From<TertiaryDeviceAttributes> for ffi::DeviceAttributesTertiary {
    fn from(value: TertiaryDeviceAttributes) -> Self {
        Self {
            unit_id: value.unit_id,
        }
    }
}

/// Color scheme reported in response to a CSI ? 996 n query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
#[allow(missing_docs, reason = "self-explanatory")]
pub enum ColorScheme {
    Light = ffi::ColorScheme::LIGHT,
    Dark = ffi::ColorScheme::DARK,
}

impl ColorScheme {
    /// Encode a color scheme report into an escape sequence.
    ///
    /// Dark color schemes emit `ESC [ ? 997 ; 1 n`, and light color schemes
    /// emit `ESC [ ? 997 ; 2 n`. The encoded bytes are identical to the
    /// terminal's internal `CSI ? 996 n` query response.
    ///
    /// Hosts should gate unsolicited sends on [`Mode::COLOR_SCHEME_REPORT`]
    /// being set.
    ///
    /// If the buffer is too small, returns [`Error::OutOfSpace`] with the
    /// required buffer size.
    pub fn encode_report(self, buf: &mut [u8]) -> Result<usize> {
        let mut written = 0;
        let result = unsafe {
            ffi::ghostty_color_scheme_report_encode(
                self.into(),
                buf.as_mut_ptr().cast(),
                buf.len(),
                &raw mut written,
            )
        };
        from_result_with_len(result, written)
    }
}

impl From<ColorScheme> for ffi::ColorScheme::Type {
    fn from(value: ColorScheme) -> Self {
        match value {
            ColorScheme::Light => ffi::ColorScheme::LIGHT,
            ColorScheme::Dark => ffi::ColorScheme::DARK,
        }
    }
}

/// Amount of compression work to perform before returning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, int_enum::IntEnum)]
#[repr(i32)]
pub enum CompressionMode {
    /// Perform one bounded compression step suitable for idle scheduling.
    Incremental = ffi::TerminalCompressionMode::INCREMENTAL,
    /// Synchronously inspect every currently eligible page.
    Full = ffi::TerminalCompressionMode::FULL,
}

/// Scheduling result from terminal compression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, int_enum::IntEnum)]
#[repr(i32)]
pub enum CompressionResult {
    /// Retained-mapping reclamation is unavailable on this target.
    Unsupported = ffi::TerminalCompressionResult::UNSUPPORTED,
    /// More incremental compression work remains.
    Pending = ffi::TerminalCompressionResult::PENDING,
    /// The pass has no continuation to schedule.
    Complete = ffi::TerminalCompressionResult::COMPLETE,
}

/// Opaque token representing a terminal's current compression activity.
///
/// The token is opaque and only equality comparisons are meaningful.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompressionActivity(u64);
