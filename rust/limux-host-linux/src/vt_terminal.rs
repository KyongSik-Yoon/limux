// VT-based terminal widget using libghostty-vt + Cairo/Pango rendering.
// This replaces the OpenGL-based terminal for systems without GL 4.3.

#![allow(dead_code)]

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::glib;

use std::cell::{Cell, RefCell};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::rc::Rc;

use limux_ghostty_vt_sys::*;

use crate::pty::{self, PtyHandle};

// -------------------------------------------------------------------
// Public API (mirrors terminal.rs interface)
// -------------------------------------------------------------------

pub struct VtTerminalWidget {
    pub overlay: gtk::Overlay,
    pub handle: VtTerminalHandle,
}

pub struct VtTerminalHandle {
    inner: Rc<VtTerminalInner>,
}

pub struct VtTerminalCallbacks {
    pub on_title_changed: Box<dyn Fn(&str)>,
    pub on_pwd_changed: Box<dyn Fn(&str)>,
    pub on_bell: Box<dyn Fn()>,
    pub on_close: Box<dyn Fn()>,
}

struct VtTerminalInner {
    terminal: Cell<GhosttyTerminal>,
    render_state: Cell<GhosttyRenderState>,
    row_iterator: Cell<GhosttyRenderStateRowIterator>,
    row_cells: Cell<GhosttyRenderStateRowCells>,
    key_event: Cell<GhosttyKeyEvent>,
    key_encoder: Cell<GhosttyKeyEncoder>,
    pty: RefCell<Option<PtyHandle>>,
    draw_area: gtk::DrawingArea,
    callbacks: RefCell<VtTerminalCallbacks>,
    cols: Cell<u16>,
    rows: Cell<u16>,
    cell_width: Cell<f64>,
    cell_height: Cell<f64>,
    cell_baseline: Cell<f64>,
    font_desc: RefCell<pango::FontDescription>,
    pty_source_id: RefCell<Option<glib::SourceId>>,
}

impl Drop for VtTerminalInner {
    fn drop(&mut self) {
        if let Some(id) = self.pty_source_id.borrow_mut().take() {
            id.remove();
        }
        unsafe {
            let t = self.terminal.get();
            if !t.is_null() {
                ghostty_terminal_free(t);
            }
            let rs = self.render_state.get();
            if !rs.is_null() {
                ghostty_render_state_free(rs);
            }
            let ri = self.row_iterator.get();
            if !ri.is_null() {
                ghostty_render_state_row_iterator_free(ri);
            }
            let rc = self.row_cells.get();
            if !rc.is_null() {
                ghostty_render_state_row_cells_free(rc);
            }
            let ke = self.key_event.get();
            if !ke.is_null() {
                ghostty_key_event_free(ke);
            }
            let enc = self.key_encoder.get();
            if !enc.is_null() {
                ghostty_key_encoder_free(enc);
            }
        }
    }
}

/// Create a VT-based terminal widget.
pub fn create_vt_terminal(
    working_directory: Option<String>,
    callbacks: VtTerminalCallbacks,
) -> Option<VtTerminalWidget> {
    // Create ghostty terminal
    let mut terminal: GhosttyTerminal = ptr::null_mut();
    let opts = GhosttyTerminalOptions {
        cols: 80,
        rows: 24,
        max_scrollback: 10_000,
    };
    let res = unsafe { ghostty_terminal_new(ptr::null(), &mut terminal, opts) };
    if res != GHOSTTY_SUCCESS {
        eprintln!("limux-vt: failed to create ghostty terminal (err={res})");
        return None;
    }

    // Create render state
    let mut render_state: GhosttyRenderState = ptr::null_mut();
    unsafe { ghostty_render_state_new(ptr::null(), &mut render_state) };

    // Create row iterator and cells (reusable)
    let mut row_iterator: GhosttyRenderStateRowIterator = ptr::null_mut();
    unsafe { ghostty_render_state_row_iterator_new(ptr::null(), &mut row_iterator) };

    let mut row_cells: GhosttyRenderStateRowCells = ptr::null_mut();
    unsafe { ghostty_render_state_row_cells_new(ptr::null(), &mut row_cells) };

    // Create key event and encoder
    let mut key_event: GhosttyKeyEvent = ptr::null_mut();
    unsafe { ghostty_key_event_new(ptr::null(), &mut key_event) };

    let mut key_encoder: GhosttyKeyEncoder = ptr::null_mut();
    unsafe { ghostty_key_encoder_new(ptr::null(), &mut key_encoder) };

    // Set default colors (dark theme)
    let bg = GhosttyColorRgb { r: 30, g: 30, b: 46 };
    let fg = GhosttyColorRgb { r: 205, g: 214, b: 244 };
    unsafe {
        ghostty_terminal_set(terminal, GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND, &bg as *const _ as *const c_void);
        ghostty_terminal_set(terminal, GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND, &fg as *const _ as *const c_void);
    }

    // Font setup
    let font_desc = pango::FontDescription::from_string("monospace 11");

    let draw_area = gtk::DrawingArea::new();
    draw_area.set_hexpand(true);
    draw_area.set_vexpand(true);
    draw_area.set_focusable(true);
    draw_area.set_can_focus(true);

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&draw_area));

    let inner = Rc::new(VtTerminalInner {
        terminal: Cell::new(terminal),
        render_state: Cell::new(render_state),
        row_iterator: Cell::new(row_iterator),
        row_cells: Cell::new(row_cells),
        key_event: Cell::new(key_event),
        key_encoder: Cell::new(key_encoder),
        pty: RefCell::new(None),
        draw_area: draw_area.clone(),
        callbacks: RefCell::new(callbacks),
        cols: Cell::new(80),
        rows: Cell::new(24),
        cell_width: Cell::new(8.0),
        cell_height: Cell::new(16.0),
        cell_baseline: Cell::new(12.0),
        font_desc: RefCell::new(font_desc),
        pty_source_id: RefCell::new(None),
    });

    // Register terminal callbacks
    setup_terminal_effects(&inner);

    // Connect draw function
    {
        let inner = inner.clone();
        draw_area.set_draw_func(move |_da, cr, width, height| {
            render_terminal(cr, width, height, &inner);
        });
    }

    // Connect resize to recompute grid
    {
        let inner = inner.clone();
        draw_area.connect_resize(move |da, width, height| {
            on_resize(da, width, height, &inner);
        });
    }

    // Keyboard input
    {
        let inner_press = inner.clone();
        let inner_release = inner.clone();
        let key_ctrl = gtk::EventControllerKey::new();

        key_ctrl.connect_key_pressed(move |_ctrl, keyval, _keycode, modifier| {
            handle_key_press(&inner_press, keyval, modifier)
        });
        key_ctrl.connect_key_released(move |_ctrl, keyval, _keycode, modifier| {
            handle_key_release(&inner_release, keyval, modifier);
        });
        draw_area.add_controller(key_ctrl);
    }

    // Mouse scroll
    {
        let inner = inner.clone();
        let scroll = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::DISCRETE,
        );
        scroll.connect_scroll(move |_ctrl, _dx, dy| {
            let terminal = inner.terminal.get();
            let delta = -(dy as isize) * 3;
            let behavior = GhosttyTerminalScrollViewport {
                tag: GHOSTTY_SCROLL_VIEWPORT_DELTA,
                value: GhosttyTerminalScrollViewportValue { delta },
            };
            unsafe { ghostty_terminal_scroll_viewport(terminal, behavior) };
            // After scroll, update render state and redraw
            unsafe {
                ghostty_render_state_update(inner.render_state.get(), terminal);
            }
            inner.draw_area.queue_draw();
            glib::Propagation::Stop
        });
        draw_area.add_controller(scroll);
    }

    // Focus
    {
        let inner = inner.clone();
        let focus_ctrl = gtk::EventControllerFocus::new();
        focus_ctrl.connect_enter(move |_| {
            // Focus gained - nothing needed for VT terminal
        });
        draw_area.add_controller(focus_ctrl);
    }

    // Spawn PTY on realize
    {
        let inner = inner.clone();
        let wd = working_directory;
        draw_area.connect_realize(move |da| {
            // Measure font metrics using widget's pango context
            measure_font_metrics(da, &inner);

            // Spawn PTY
            let shell = pty::default_shell();
            let cols = inner.cols.get();
            let rows = inner.rows.get();
            let cw = inner.cell_width.get() as u16;
            let ch = inner.cell_height.get() as u16;

            match PtyHandle::spawn(
                &shell,
                wd.as_deref(),
                cols,
                rows,
                cw,
                ch,
            ) {
                Ok(pty_handle) => {
                    // Set up PTY read callback
                    let fd = pty_handle.raw_fd();
                    *inner.pty.borrow_mut() = Some(pty_handle);

                    // Poll PTY for data at ~120Hz using a GLib timer.
                    let inner_read = inner.clone();
                    let source_id = glib::timeout_add_local(
                        std::time::Duration::from_millis(8),
                        move || {
                            on_pty_readable(&inner_read, glib::IOCondition::IN)
                        },
                    );
                    *inner.pty_source_id.borrow_mut() = Some(source_id);
                }
                Err(e) => {
                    eprintln!("limux-vt: failed to spawn PTY: {e}");
                }
            }
        });
    }

    Some(VtTerminalWidget {
        overlay,
        handle: VtTerminalHandle { inner },
    })
}

// -------------------------------------------------------------------
// Terminal effects (callbacks from VT processing)
// -------------------------------------------------------------------

fn setup_terminal_effects(inner: &Rc<VtTerminalInner>) {
    let terminal = inner.terminal.get();

    // Store inner pointer as userdata for callbacks
    let userdata = Rc::into_raw(inner.clone()) as *mut c_void;
    unsafe {
        ghostty_terminal_set(terminal, GHOSTTY_TERMINAL_OPT_USERDATA, userdata);
    }

    // write_pty: terminal query responses go back to the PTY
    unsafe extern "C" fn write_pty_cb(
        _terminal: GhosttyTerminal,
        userdata: *mut c_void,
        data: *const u8,
        len: usize,
    ) {
        let inner = &*(userdata as *const VtTerminalInner);
        if let Some(ref pty) = *inner.pty.borrow() {
            let bytes = std::slice::from_raw_parts(data, len);
            let _ = pty.write(bytes);
        }
    }

    // bell callback
    unsafe extern "C" fn bell_cb(_terminal: GhosttyTerminal, userdata: *mut c_void) {
        let inner = &*(userdata as *const VtTerminalInner);
        let callbacks = inner.callbacks.borrow();
        (callbacks.on_bell)();
    }

    // title changed callback
    unsafe extern "C" fn title_changed_cb(terminal: GhosttyTerminal, userdata: *mut c_void) {
        let inner = &*(userdata as *const VtTerminalInner);
        let mut title_str = GhosttyString { ptr: ptr::null(), len: 0 };
        let res = ghostty_terminal_get(
            terminal,
            GHOSTTY_TERMINAL_DATA_TITLE,
            &mut title_str as *mut _ as *mut c_void,
        );
        if res == GHOSTTY_SUCCESS && !title_str.ptr.is_null() && title_str.len > 0 {
            let bytes = std::slice::from_raw_parts(title_str.ptr, title_str.len);
            if let Ok(title) = std::str::from_utf8(bytes) {
                let callbacks = inner.callbacks.borrow();
                (callbacks.on_title_changed)(title);
            }
        }
    }

    unsafe {
        ghostty_terminal_set(
            terminal,
            GHOSTTY_TERMINAL_OPT_WRITE_PTY,
            write_pty_cb as *const c_void,
        );
        ghostty_terminal_set(
            terminal,
            GHOSTTY_TERMINAL_OPT_BELL,
            bell_cb as *const c_void,
        );
        ghostty_terminal_set(
            terminal,
            GHOSTTY_TERMINAL_OPT_TITLE_CHANGED,
            title_changed_cb as *const c_void,
        );
    }
}

// -------------------------------------------------------------------
// Font metrics
// -------------------------------------------------------------------

fn measure_font_metrics(da: &gtk::DrawingArea, inner: &VtTerminalInner) {
    let pango_ctx = da.pango_context();
    let font_desc = inner.font_desc.borrow();
    let metrics = pango_ctx.metrics(Some(&font_desc), None);

    let cell_width = metrics.approximate_char_width() as f64 / pango::SCALE as f64;
    let ascent = metrics.ascent() as f64 / pango::SCALE as f64;
    let descent = metrics.descent() as f64 / pango::SCALE as f64;
    let cell_height = ascent + descent;

    inner.cell_width.set(cell_width);
    inner.cell_height.set(cell_height);
    inner.cell_baseline.set(ascent);
}

// -------------------------------------------------------------------
// PTY I/O
// -------------------------------------------------------------------

fn on_pty_readable(inner: &VtTerminalInner, condition: glib::IOCondition) -> glib::ControlFlow {
    if condition.contains(glib::IOCondition::HUP) {
        // Child exited
        let callbacks = inner.callbacks.borrow();
        (callbacks.on_close)();
        return glib::ControlFlow::Break;
    }

    let mut buf = [0u8; 65536];
    let pty_ref = inner.pty.borrow();
    let Some(ref pty) = *pty_ref else {
        return glib::ControlFlow::Break;
    };

    loop {
        match pty.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let terminal = inner.terminal.get();
                unsafe {
                    ghostty_terminal_vt_write(terminal, buf.as_ptr(), n);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => {
                return glib::ControlFlow::Break;
            }
        }
    }

    // Update render state and request redraw
    unsafe {
        ghostty_render_state_update(inner.render_state.get(), inner.terminal.get());
    }
    inner.draw_area.queue_draw();

    glib::ControlFlow::Continue
}

// -------------------------------------------------------------------
// Resize
// -------------------------------------------------------------------

fn on_resize(da: &gtk::DrawingArea, width: i32, height: i32, inner: &VtTerminalInner) {
    if width <= 0 || height <= 0 {
        return;
    }

    // Remeasure font in case DPI changed
    measure_font_metrics(da, inner);

    let cw = inner.cell_width.get();
    let ch = inner.cell_height.get();
    if cw <= 0.0 || ch <= 0.0 {
        return;
    }

    let new_cols = (width as f64 / cw).floor().max(1.0) as u16;
    let new_rows = (height as f64 / ch).floor().max(1.0) as u16;

    if new_cols == inner.cols.get() && new_rows == inner.rows.get() {
        return;
    }

    inner.cols.set(new_cols);
    inner.rows.set(new_rows);

    let terminal = inner.terminal.get();
    unsafe {
        ghostty_terminal_resize(
            terminal,
            new_cols,
            new_rows,
            cw as u32,
            ch as u32,
        );
        ghostty_render_state_update(inner.render_state.get(), terminal);
    }

    // Resize PTY
    if let Some(ref pty) = *inner.pty.borrow() {
        let _ = pty.resize(new_cols, new_rows, cw as u16, ch as u16);
    }
}

// -------------------------------------------------------------------
// Rendering (Cairo + Pango)
// -------------------------------------------------------------------

fn render_terminal(cr: &cairo::Context, width: i32, height: i32, inner: &VtTerminalInner) {
    let rs = inner.render_state.get();
    if rs.is_null() {
        return;
    }

    let cw = inner.cell_width.get();
    let ch = inner.cell_height.get();
    let baseline = inner.cell_baseline.get();

    // Get colors
    let mut colors = GhosttyRenderStateColors {
        size: std::mem::size_of::<GhosttyRenderStateColors>(),
        background: GhosttyColorRgb::default(),
        foreground: GhosttyColorRgb::default(),
        cursor: GhosttyColorRgb::default(),
        cursor_has_value: false,
        palette: [GhosttyColorRgb::default(); 256],
    };
    unsafe { ghostty_render_state_colors_get(rs, &mut colors) };

    // Clear with background
    let bg = &colors.background;
    cr.set_source_rgb(bg.r as f64 / 255.0, bg.g as f64 / 255.0, bg.b as f64 / 255.0);
    cr.paint().ok();

    // Populate row iterator from render state
    let ri = inner.row_iterator.get();
    unsafe {
        ghostty_render_state_get(
            rs,
            GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
            ri as *mut c_void,
        );
    }

    let cells_handle = inner.row_cells.get();
    let pango_ctx = inner.draw_area.pango_context();
    let font_desc = inner.font_desc.borrow();

    let mut row_idx: u16 = 0;

    while unsafe { ghostty_render_state_row_iterator_next(ri) } {
        let y = row_idx as f64 * ch;
        if y > height as f64 {
            break;
        }

        // Get cells for this row
        unsafe {
            ghostty_render_state_row_get(
                ri,
                GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                cells_handle as *mut c_void,
            );
        }

        let mut col: u16 = 0;
        while unsafe { ghostty_render_state_row_cells_next(cells_handle) } {
            let x = col as f64 * cw;

            // Get cell background color
            let mut cell_bg = GhosttyColorRgb::default();
            let bg_res = unsafe {
                ghostty_render_state_row_cells_get(
                    cells_handle,
                    GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR,
                    &mut cell_bg as *mut _ as *mut c_void,
                )
            };
            if bg_res == GHOSTTY_SUCCESS {
                cr.set_source_rgb(
                    cell_bg.r as f64 / 255.0,
                    cell_bg.g as f64 / 255.0,
                    cell_bg.b as f64 / 255.0,
                );
                cr.rectangle(x, y, cw, ch);
                cr.fill().ok();
            }

            // Get grapheme codepoints
            let mut graphemes_len: u32 = 0;
            unsafe {
                ghostty_render_state_row_cells_get(
                    cells_handle,
                    GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                    &mut graphemes_len as *mut _ as *mut c_void,
                );
            }

            if graphemes_len > 0 {
                let mut codepoints = vec![0u32; graphemes_len as usize];
                unsafe {
                    ghostty_render_state_row_cells_get(
                        cells_handle,
                        GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                        codepoints.as_mut_ptr() as *mut c_void,
                    );
                }

                // Build UTF-8 string from codepoints
                let text: String = codepoints.iter()
                    .filter_map(|&cp| char::from_u32(cp))
                    .collect();

                if !text.is_empty() && text != " " {
                    // Get foreground color
                    let mut cell_fg = colors.foreground;
                    let fg_res = unsafe {
                        ghostty_render_state_row_cells_get(
                            cells_handle,
                            GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR,
                            &mut cell_fg as *mut _ as *mut c_void,
                        )
                    };
                    if fg_res != GHOSTTY_SUCCESS {
                        cell_fg = colors.foreground;
                    }

                    // Get style for bold/italic
                    let mut style = unsafe { std::mem::zeroed::<GhosttyStyle>() };
                    style.size = std::mem::size_of::<GhosttyStyle>();
                    unsafe {
                        ghostty_render_state_row_cells_get(
                            cells_handle,
                            GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,
                            &mut style as *mut _ as *mut c_void,
                        );
                    }

                    // Create pango layout for this cell
                    let layout = pango::Layout::new(&pango_ctx);
                    let mut fd = font_desc.clone();
                    if style.bold {
                        fd.set_weight(pango::Weight::Bold);
                    }
                    if style.italic {
                        fd.set_style(pango::Style::Italic);
                    }
                    layout.set_font_description(Some(&fd));
                    layout.set_text(&text);

                    cr.set_source_rgb(
                        cell_fg.r as f64 / 255.0,
                        cell_fg.g as f64 / 255.0,
                        cell_fg.b as f64 / 255.0,
                    );
                    cr.move_to(x, y);
                    pangocairo::functions::show_layout(cr, &layout);
                }
            }

            col += 1;
        }

        // Reset row dirty
        let dirty_false = false;
        unsafe {
            ghostty_render_state_row_set(
                ri,
                GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY,
                &dirty_false as *const _ as *const c_void,
            );
        }

        row_idx += 1;
    }

    // Draw cursor
    render_cursor(cr, rs, cw, ch, &colors);

    // Reset global dirty state
    let dirty_false = GHOSTTY_RENDER_STATE_DIRTY_FALSE;
    unsafe {
        ghostty_render_state_set(
            rs,
            GHOSTTY_RENDER_STATE_OPTION_DIRTY,
            &dirty_false as *const _ as *const c_void,
        );
    }
}

fn render_cursor(
    cr: &cairo::Context,
    rs: GhosttyRenderState,
    cw: f64,
    ch: f64,
    colors: &GhosttyRenderStateColors,
) {
    let mut has_cursor = false;
    unsafe {
        ghostty_render_state_get(
            rs,
            GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE,
            &mut has_cursor as *mut _ as *mut c_void,
        );
    }
    if !has_cursor {
        return;
    }

    let mut visible = true;
    unsafe {
        ghostty_render_state_get(
            rs,
            GHOSTTY_RENDER_STATE_DATA_CURSOR_VISIBLE,
            &mut visible as *mut _ as *mut c_void,
        );
    }
    if !visible {
        return;
    }

    let mut cx: u16 = 0;
    let mut cy: u16 = 0;
    unsafe {
        ghostty_render_state_get(
            rs,
            GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X,
            &mut cx as *mut _ as *mut c_void,
        );
        ghostty_render_state_get(
            rs,
            GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y,
            &mut cy as *mut _ as *mut c_void,
        );
    }

    let cursor_color = if colors.cursor_has_value {
        &colors.cursor
    } else {
        &colors.foreground
    };

    let mut style: GhosttyRenderStateCursorVisualStyle = GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK;
    unsafe {
        ghostty_render_state_get(
            rs,
            GHOSTTY_RENDER_STATE_DATA_CURSOR_VISUAL_STYLE,
            &mut style as *mut _ as *mut c_void,
        );
    }

    let x = cx as f64 * cw;
    let y = cy as f64 * ch;

    cr.set_source_rgb(
        cursor_color.r as f64 / 255.0,
        cursor_color.g as f64 / 255.0,
        cursor_color.b as f64 / 255.0,
    );

    match style {
        GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK => {
            cr.rectangle(x, y, cw, ch);
            cr.fill().ok();
        }
        GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR => {
            cr.rectangle(x, y, 2.0, ch);
            cr.fill().ok();
        }
        GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE => {
            cr.rectangle(x, y + ch - 2.0, cw, 2.0);
            cr.fill().ok();
        }
        GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW => {
            cr.set_line_width(1.0);
            cr.rectangle(x + 0.5, y + 0.5, cw - 1.0, ch - 1.0);
            cr.stroke().ok();
        }
        _ => {}
    }
}

// -------------------------------------------------------------------
// Keyboard input
// -------------------------------------------------------------------

fn handle_key_press(inner: &VtTerminalInner, keyval: gtk::gdk::Key, modifier: gtk::gdk::ModifierType) -> glib::Propagation {
    let terminal = inner.terminal.get();
    let encoder = inner.key_encoder.get();
    let event = inner.key_event.get();

    // Sync encoder with terminal modes
    unsafe { ghostty_key_encoder_setopt_from_terminal(encoder, terminal) };

    // Map GTK keyval to GhosttyKey
    let key = gtk_keyval_to_ghostty_key(keyval);

    // Map modifiers
    let mut mods: GhosttyMods = 0;
    if modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK) { mods |= GHOSTTY_MODS_SHIFT; }
    if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) { mods |= GHOSTTY_MODS_CTRL; }
    if modifier.contains(gtk::gdk::ModifierType::ALT_MASK) { mods |= GHOSTTY_MODS_ALT; }
    if modifier.contains(gtk::gdk::ModifierType::SUPER_MASK) { mods |= GHOSTTY_MODS_SUPER; }

    unsafe {
        ghostty_key_event_set_action(event, GHOSTTY_KEY_ACTION_PRESS);
        ghostty_key_event_set_key(event, key);
        ghostty_key_event_set_mods(event, mods);
        ghostty_key_event_set_consumed_mods(event, 0);
        ghostty_key_event_set_composing(event, false);
    }

    // Set UTF-8 text from keyval
    if let Some(ch) = keyval.to_unicode() {
        if !ch.is_control() {
            let mut utf8_buf = [0u8; 4];
            let utf8 = ch.encode_utf8(&mut utf8_buf);
            unsafe {
                ghostty_key_event_set_utf8(event, utf8.as_ptr() as *const c_char, utf8.len());
                ghostty_key_event_set_unshifted_codepoint(event, ch as u32);
            }
        } else {
            unsafe {
                ghostty_key_event_set_utf8(event, ptr::null(), 0);
                ghostty_key_event_set_unshifted_codepoint(event, 0);
            }
        }
    } else {
        unsafe {
            ghostty_key_event_set_utf8(event, ptr::null(), 0);
            ghostty_key_event_set_unshifted_codepoint(event, 0);
        }
    }

    // Encode the key event
    let mut buf = [0u8; 128];
    let mut written: usize = 0;
    let res = unsafe {
        ghostty_key_encoder_encode(
            encoder,
            event,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            &mut written,
        )
    };

    if res == GHOSTTY_SUCCESS && written > 0 {
        if let Some(ref pty) = *inner.pty.borrow() {
            let _ = pty.write(&buf[..written]);
        }
        return glib::Propagation::Stop;
    }

    glib::Propagation::Proceed
}

fn handle_key_release(inner: &VtTerminalInner, keyval: gtk::gdk::Key, modifier: gtk::gdk::ModifierType) {
    // Release events generally don't produce output in legacy mode,
    // but we send them for Kitty protocol support
    let terminal = inner.terminal.get();
    let encoder = inner.key_encoder.get();
    let event = inner.key_event.get();

    unsafe { ghostty_key_encoder_setopt_from_terminal(encoder, terminal) };

    let key = gtk_keyval_to_ghostty_key(keyval);
    let mut mods: GhosttyMods = 0;
    if modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK) { mods |= GHOSTTY_MODS_SHIFT; }
    if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) { mods |= GHOSTTY_MODS_CTRL; }
    if modifier.contains(gtk::gdk::ModifierType::ALT_MASK) { mods |= GHOSTTY_MODS_ALT; }
    if modifier.contains(gtk::gdk::ModifierType::SUPER_MASK) { mods |= GHOSTTY_MODS_SUPER; }

    unsafe {
        ghostty_key_event_set_action(event, GHOSTTY_KEY_ACTION_RELEASE);
        ghostty_key_event_set_key(event, key);
        ghostty_key_event_set_mods(event, mods);
        ghostty_key_event_set_utf8(event, ptr::null(), 0);
    }

    let mut buf = [0u8; 128];
    let mut written: usize = 0;
    let res = unsafe {
        ghostty_key_encoder_encode(
            encoder,
            event,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            &mut written,
        )
    };

    if res == GHOSTTY_SUCCESS && written > 0 {
        if let Some(ref pty) = *inner.pty.borrow() {
            let _ = pty.write(&buf[..written]);
        }
    }
}

// -------------------------------------------------------------------
// GTK keyval → GhosttyKey mapping
// -------------------------------------------------------------------

fn gtk_keyval_to_ghostty_key(keyval: gtk::gdk::Key) -> GhosttyKey {
    use gtk::gdk::Key as K;

    match keyval {
        K::a | K::A => GHOSTTY_KEY_A,
        K::b | K::B => GHOSTTY_KEY_B,
        K::c | K::C => GHOSTTY_KEY_C,
        K::d | K::D => GHOSTTY_KEY_D,
        K::e | K::E => GHOSTTY_KEY_E,
        K::f | K::F => GHOSTTY_KEY_F,
        K::g | K::G => GHOSTTY_KEY_G,
        K::h | K::H => GHOSTTY_KEY_H,
        K::i | K::I => GHOSTTY_KEY_I,
        K::j | K::J => GHOSTTY_KEY_J,
        K::k | K::K => GHOSTTY_KEY_K,
        K::l | K::L => GHOSTTY_KEY_L,
        K::m | K::M => GHOSTTY_KEY_M,
        K::n | K::N => GHOSTTY_KEY_N,
        K::o | K::O => GHOSTTY_KEY_O,
        K::p | K::P => GHOSTTY_KEY_P,
        K::q | K::Q => GHOSTTY_KEY_Q,
        K::r | K::R => GHOSTTY_KEY_R,
        K::s | K::S => GHOSTTY_KEY_S,
        K::t | K::T => GHOSTTY_KEY_T,
        K::u | K::U => GHOSTTY_KEY_U,
        K::v | K::V => GHOSTTY_KEY_V,
        K::w | K::W => GHOSTTY_KEY_W,
        K::x | K::X => GHOSTTY_KEY_X,
        K::y | K::Y => GHOSTTY_KEY_Y,
        K::z | K::Z => GHOSTTY_KEY_Z,
        K::_0 | K::parenright => GHOSTTY_KEY_DIGIT_0,
        K::_1 | K::exclam => GHOSTTY_KEY_DIGIT_1,
        K::_2 | K::at => GHOSTTY_KEY_DIGIT_2,
        K::_3 | K::numbersign => GHOSTTY_KEY_DIGIT_3,
        K::_4 | K::dollar => GHOSTTY_KEY_DIGIT_4,
        K::_5 | K::percent => GHOSTTY_KEY_DIGIT_5,
        K::_6 | K::asciicircum => GHOSTTY_KEY_DIGIT_6,
        K::_7 | K::ampersand => GHOSTTY_KEY_DIGIT_7,
        K::_8 | K::asterisk => GHOSTTY_KEY_DIGIT_8,
        K::_9 | K::parenleft => GHOSTTY_KEY_DIGIT_9,
        K::space => GHOSTTY_KEY_SPACE,
        K::Return | K::KP_Enter => GHOSTTY_KEY_ENTER,
        K::Tab | K::ISO_Left_Tab => GHOSTTY_KEY_TAB,
        K::BackSpace => GHOSTTY_KEY_BACKSPACE,
        K::Escape => GHOSTTY_KEY_ESCAPE,
        K::Delete | K::KP_Delete => GHOSTTY_KEY_DELETE,
        K::Insert | K::KP_Insert => GHOSTTY_KEY_INSERT,
        K::Home | K::KP_Home => GHOSTTY_KEY_HOME,
        K::End | K::KP_End => GHOSTTY_KEY_END,
        K::Page_Up | K::KP_Page_Up => GHOSTTY_KEY_PAGE_UP,
        K::Page_Down | K::KP_Page_Down => GHOSTTY_KEY_PAGE_DOWN,
        K::Up | K::KP_Up => GHOSTTY_KEY_ARROW_UP,
        K::Down | K::KP_Down => GHOSTTY_KEY_ARROW_DOWN,
        K::Left | K::KP_Left => GHOSTTY_KEY_ARROW_LEFT,
        K::Right | K::KP_Right => GHOSTTY_KEY_ARROW_RIGHT,
        K::F1 => GHOSTTY_KEY_F1,
        K::F2 => GHOSTTY_KEY_F2,
        K::F3 => GHOSTTY_KEY_F3,
        K::F4 => GHOSTTY_KEY_F4,
        K::F5 => GHOSTTY_KEY_F5,
        K::F6 => GHOSTTY_KEY_F6,
        K::F7 => GHOSTTY_KEY_F7,
        K::F8 => GHOSTTY_KEY_F8,
        K::F9 => GHOSTTY_KEY_F9,
        K::F10 => GHOSTTY_KEY_F10,
        K::F11 => GHOSTTY_KEY_F11,
        K::F12 => GHOSTTY_KEY_F12,
        K::minus | K::underscore => GHOSTTY_KEY_MINUS,
        K::equal | K::plus => GHOSTTY_KEY_EQUAL,
        K::bracketleft | K::braceleft => GHOSTTY_KEY_BRACKET_LEFT,
        K::bracketright | K::braceright => GHOSTTY_KEY_BRACKET_RIGHT,
        K::backslash | K::bar => GHOSTTY_KEY_BACKSLASH,
        K::semicolon | K::colon => GHOSTTY_KEY_SEMICOLON,
        K::apostrophe | K::quotedbl => GHOSTTY_KEY_QUOTE,
        K::comma | K::less => GHOSTTY_KEY_COMMA,
        K::period | K::greater => GHOSTTY_KEY_PERIOD,
        K::slash | K::question => GHOSTTY_KEY_SLASH,
        K::grave | K::asciitilde => GHOSTTY_KEY_BACKQUOTE,
        _ => GHOSTTY_KEY_UNIDENTIFIED,
    }
}

use gtk::cairo;
use gtk::pango;
use pangocairo;
