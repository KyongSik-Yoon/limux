#![allow(non_camel_case_types, non_upper_case_globals, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

// -------------------------------------------------------------------
// Result codes
// -------------------------------------------------------------------

pub type GhosttyResult = c_int;
pub const GHOSTTY_SUCCESS: GhosttyResult = 0;
pub const GHOSTTY_OUT_OF_MEMORY: GhosttyResult = -1;
pub const GHOSTTY_INVALID_VALUE: GhosttyResult = -2;
pub const GHOSTTY_OUT_OF_SPACE: GhosttyResult = -3;
pub const GHOSTTY_NO_VALUE: GhosttyResult = -4;

// -------------------------------------------------------------------
// Borrowed string
// -------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GhosttyString {
    pub ptr: *const u8,
    pub len: usize,
}

// -------------------------------------------------------------------
// Color types
// -------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GhosttyColorRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub type GhosttyColorPaletteIndex = u8;

// -------------------------------------------------------------------
// Style types
// -------------------------------------------------------------------

pub type GhosttyStyleId = u16;

pub type GhosttyStyleColorTag = c_int;
pub const GHOSTTY_STYLE_COLOR_NONE: GhosttyStyleColorTag = 0;
pub const GHOSTTY_STYLE_COLOR_PALETTE: GhosttyStyleColorTag = 1;
pub const GHOSTTY_STYLE_COLOR_RGB: GhosttyStyleColorTag = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub union GhosttyStyleColorValue {
    pub palette: GhosttyColorPaletteIndex,
    pub rgb: GhosttyColorRgb,
    pub _padding: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GhosttyStyleColor {
    pub tag: GhosttyStyleColorTag,
    pub value: GhosttyStyleColorValue,
}

#[repr(C)]
pub struct GhosttyStyle {
    pub size: usize,
    pub fg_color: GhosttyStyleColor,
    pub bg_color: GhosttyStyleColor,
    pub underline_color: GhosttyStyleColor,
    pub bold: bool,
    pub italic: bool,
    pub faint: bool,
    pub blink: bool,
    pub inverse: bool,
    pub invisible: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub underline: c_int,
}

// -------------------------------------------------------------------
// Terminal opaque handle
// -------------------------------------------------------------------

pub type GhosttyTerminal = *mut c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GhosttyTerminalOptions {
    pub cols: u16,
    pub rows: u16,
    pub max_scrollback: usize,
}

// Terminal scroll viewport
pub type GhosttyTerminalScrollViewportTag = c_int;
pub const GHOSTTY_SCROLL_VIEWPORT_TOP: GhosttyTerminalScrollViewportTag = 0;
pub const GHOSTTY_SCROLL_VIEWPORT_BOTTOM: GhosttyTerminalScrollViewportTag = 1;
pub const GHOSTTY_SCROLL_VIEWPORT_DELTA: GhosttyTerminalScrollViewportTag = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub union GhosttyTerminalScrollViewportValue {
    pub delta: isize,
    pub _padding: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GhosttyTerminalScrollViewport {
    pub tag: GhosttyTerminalScrollViewportTag,
    pub value: GhosttyTerminalScrollViewportValue,
}

// Terminal screen
pub type GhosttyTerminalScreen = c_int;
pub const GHOSTTY_TERMINAL_SCREEN_PRIMARY: GhosttyTerminalScreen = 0;
pub const GHOSTTY_TERMINAL_SCREEN_ALTERNATE: GhosttyTerminalScreen = 1;

// Terminal option identifiers
pub type GhosttyTerminalOption = c_int;
pub const GHOSTTY_TERMINAL_OPT_USERDATA: GhosttyTerminalOption = 0;
pub const GHOSTTY_TERMINAL_OPT_WRITE_PTY: GhosttyTerminalOption = 1;
pub const GHOSTTY_TERMINAL_OPT_BELL: GhosttyTerminalOption = 2;
pub const GHOSTTY_TERMINAL_OPT_ENQUIRY: GhosttyTerminalOption = 3;
pub const GHOSTTY_TERMINAL_OPT_XTVERSION: GhosttyTerminalOption = 4;
pub const GHOSTTY_TERMINAL_OPT_TITLE_CHANGED: GhosttyTerminalOption = 5;
pub const GHOSTTY_TERMINAL_OPT_SIZE: GhosttyTerminalOption = 6;
pub const GHOSTTY_TERMINAL_OPT_COLOR_SCHEME: GhosttyTerminalOption = 7;
pub const GHOSTTY_TERMINAL_OPT_DEVICE_ATTRIBUTES: GhosttyTerminalOption = 8;
pub const GHOSTTY_TERMINAL_OPT_TITLE: GhosttyTerminalOption = 9;
pub const GHOSTTY_TERMINAL_OPT_PWD: GhosttyTerminalOption = 10;
pub const GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND: GhosttyTerminalOption = 11;
pub const GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND: GhosttyTerminalOption = 12;
pub const GHOSTTY_TERMINAL_OPT_COLOR_CURSOR: GhosttyTerminalOption = 13;
pub const GHOSTTY_TERMINAL_OPT_COLOR_PALETTE: GhosttyTerminalOption = 14;

// Terminal data types for ghostty_terminal_get
pub type GhosttyTerminalData = c_int;
pub const GHOSTTY_TERMINAL_DATA_COLS: GhosttyTerminalData = 1;
pub const GHOSTTY_TERMINAL_DATA_ROWS: GhosttyTerminalData = 2;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_X: GhosttyTerminalData = 3;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_Y: GhosttyTerminalData = 4;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_PENDING_WRAP: GhosttyTerminalData = 5;
pub const GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN: GhosttyTerminalData = 6;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_VISIBLE: GhosttyTerminalData = 7;
pub const GHOSTTY_TERMINAL_DATA_KITTY_KEYBOARD_FLAGS: GhosttyTerminalData = 8;
pub const GHOSTTY_TERMINAL_DATA_SCROLLBAR: GhosttyTerminalData = 9;
pub const GHOSTTY_TERMINAL_DATA_MOUSE_TRACKING: GhosttyTerminalData = 11;
pub const GHOSTTY_TERMINAL_DATA_TITLE: GhosttyTerminalData = 12;
pub const GHOSTTY_TERMINAL_DATA_PWD: GhosttyTerminalData = 13;
pub const GHOSTTY_TERMINAL_DATA_TOTAL_ROWS: GhosttyTerminalData = 14;
pub const GHOSTTY_TERMINAL_DATA_SCROLLBACK_ROWS: GhosttyTerminalData = 15;
pub const GHOSTTY_TERMINAL_DATA_COLOR_FOREGROUND: GhosttyTerminalData = 18;
pub const GHOSTTY_TERMINAL_DATA_COLOR_BACKGROUND: GhosttyTerminalData = 19;

// Terminal callbacks
pub type GhosttyTerminalWritePtyFn =
    unsafe extern "C" fn(GhosttyTerminal, *mut c_void, *const u8, usize);
pub type GhosttyTerminalBellFn = unsafe extern "C" fn(GhosttyTerminal, *mut c_void);
pub type GhosttyTerminalTitleChangedFn = unsafe extern "C" fn(GhosttyTerminal, *mut c_void);

// -------------------------------------------------------------------
// Render state
// -------------------------------------------------------------------

pub type GhosttyRenderState = *mut c_void;
pub type GhosttyRenderStateRowIterator = *mut c_void;
pub type GhosttyRenderStateRowCells = *mut c_void;

pub type GhosttyRenderStateDirty = c_int;
pub const GHOSTTY_RENDER_STATE_DIRTY_FALSE: GhosttyRenderStateDirty = 0;
pub const GHOSTTY_RENDER_STATE_DIRTY_PARTIAL: GhosttyRenderStateDirty = 1;
pub const GHOSTTY_RENDER_STATE_DIRTY_FULL: GhosttyRenderStateDirty = 2;

pub type GhosttyRenderStateCursorVisualStyle = c_int;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR: GhosttyRenderStateCursorVisualStyle = 0;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK: GhosttyRenderStateCursorVisualStyle = 1;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE: GhosttyRenderStateCursorVisualStyle =
    2;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW:
    GhosttyRenderStateCursorVisualStyle = 3;

// Render state data queries
pub type GhosttyRenderStateData = c_int;
pub const GHOSTTY_RENDER_STATE_DATA_COLS: GhosttyRenderStateData = 1;
pub const GHOSTTY_RENDER_STATE_DATA_ROWS: GhosttyRenderStateData = 2;
pub const GHOSTTY_RENDER_STATE_DATA_DIRTY: GhosttyRenderStateData = 3;
pub const GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR: GhosttyRenderStateData = 4;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_BACKGROUND: GhosttyRenderStateData = 5;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_FOREGROUND: GhosttyRenderStateData = 6;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_CURSOR: GhosttyRenderStateData = 7;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_CURSOR_HAS_VALUE: GhosttyRenderStateData = 8;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_PALETTE: GhosttyRenderStateData = 9;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VISUAL_STYLE: GhosttyRenderStateData = 10;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VISIBLE: GhosttyRenderStateData = 11;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_BLINKING: GhosttyRenderStateData = 12;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE: GhosttyRenderStateData = 14;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X: GhosttyRenderStateData = 15;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y: GhosttyRenderStateData = 16;

pub type GhosttyRenderStateOption = c_int;
pub const GHOSTTY_RENDER_STATE_OPTION_DIRTY: GhosttyRenderStateOption = 0;

// Row data queries
pub type GhosttyRenderStateRowData = c_int;
pub const GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY: GhosttyRenderStateRowData = 1;
pub const GHOSTTY_RENDER_STATE_ROW_DATA_CELLS: GhosttyRenderStateRowData = 3;

pub type GhosttyRenderStateRowOption = c_int;
pub const GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY: GhosttyRenderStateRowOption = 0;

// Cell data queries
pub type GhosttyRenderStateRowCellsData = c_int;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE: GhosttyRenderStateRowCellsData = 2;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN: GhosttyRenderStateRowCellsData = 3;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF: GhosttyRenderStateRowCellsData = 4;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR: GhosttyRenderStateRowCellsData = 5;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR: GhosttyRenderStateRowCellsData = 6;

#[repr(C)]
pub struct GhosttyRenderStateColors {
    pub size: usize,
    pub background: GhosttyColorRgb,
    pub foreground: GhosttyColorRgb,
    pub cursor: GhosttyColorRgb,
    pub cursor_has_value: bool,
    pub palette: [GhosttyColorRgb; 256],
}

// -------------------------------------------------------------------
// Key event and encoder
// -------------------------------------------------------------------

pub type GhosttyKeyEvent = *mut c_void;
pub type GhosttyKeyEncoder = *mut c_void;

pub type GhosttyKeyAction = c_int;
pub const GHOSTTY_KEY_ACTION_RELEASE: GhosttyKeyAction = 0;
pub const GHOSTTY_KEY_ACTION_PRESS: GhosttyKeyAction = 1;
pub const GHOSTTY_KEY_ACTION_REPEAT: GhosttyKeyAction = 2;

pub type GhosttyMods = u16;
pub const GHOSTTY_MODS_SHIFT: GhosttyMods = 1 << 0;
pub const GHOSTTY_MODS_CTRL: GhosttyMods = 1 << 1;
pub const GHOSTTY_MODS_ALT: GhosttyMods = 1 << 2;
pub const GHOSTTY_MODS_SUPER: GhosttyMods = 1 << 3;

pub type GhosttyKey = c_int;
pub const GHOSTTY_KEY_UNIDENTIFIED: GhosttyKey = 0;
pub const GHOSTTY_KEY_BACKQUOTE: GhosttyKey = 1;
pub const GHOSTTY_KEY_BACKSLASH: GhosttyKey = 2;
pub const GHOSTTY_KEY_BRACKET_LEFT: GhosttyKey = 3;
pub const GHOSTTY_KEY_BRACKET_RIGHT: GhosttyKey = 4;
pub const GHOSTTY_KEY_COMMA: GhosttyKey = 5;
pub const GHOSTTY_KEY_DIGIT_0: GhosttyKey = 6;
pub const GHOSTTY_KEY_DIGIT_1: GhosttyKey = 7;
pub const GHOSTTY_KEY_DIGIT_2: GhosttyKey = 8;
pub const GHOSTTY_KEY_DIGIT_3: GhosttyKey = 9;
pub const GHOSTTY_KEY_DIGIT_4: GhosttyKey = 10;
pub const GHOSTTY_KEY_DIGIT_5: GhosttyKey = 11;
pub const GHOSTTY_KEY_DIGIT_6: GhosttyKey = 12;
pub const GHOSTTY_KEY_DIGIT_7: GhosttyKey = 13;
pub const GHOSTTY_KEY_DIGIT_8: GhosttyKey = 14;
pub const GHOSTTY_KEY_DIGIT_9: GhosttyKey = 15;
pub const GHOSTTY_KEY_EQUAL: GhosttyKey = 16;
pub const GHOSTTY_KEY_INTL_BACKSLASH: GhosttyKey = 17;
pub const GHOSTTY_KEY_INTL_RO: GhosttyKey = 18;
pub const GHOSTTY_KEY_INTL_YEN: GhosttyKey = 19;
pub const GHOSTTY_KEY_A: GhosttyKey = 20;
pub const GHOSTTY_KEY_B: GhosttyKey = 21;
pub const GHOSTTY_KEY_C: GhosttyKey = 22;
pub const GHOSTTY_KEY_D: GhosttyKey = 23;
pub const GHOSTTY_KEY_E: GhosttyKey = 24;
pub const GHOSTTY_KEY_F: GhosttyKey = 25;
pub const GHOSTTY_KEY_G: GhosttyKey = 26;
pub const GHOSTTY_KEY_H: GhosttyKey = 27;
pub const GHOSTTY_KEY_I: GhosttyKey = 28;
pub const GHOSTTY_KEY_J: GhosttyKey = 29;
pub const GHOSTTY_KEY_K: GhosttyKey = 30;
pub const GHOSTTY_KEY_L: GhosttyKey = 31;
pub const GHOSTTY_KEY_M: GhosttyKey = 32;
pub const GHOSTTY_KEY_N: GhosttyKey = 33;
pub const GHOSTTY_KEY_O: GhosttyKey = 34;
pub const GHOSTTY_KEY_P: GhosttyKey = 35;
pub const GHOSTTY_KEY_Q: GhosttyKey = 36;
pub const GHOSTTY_KEY_R: GhosttyKey = 37;
pub const GHOSTTY_KEY_S: GhosttyKey = 38;
pub const GHOSTTY_KEY_T: GhosttyKey = 39;
pub const GHOSTTY_KEY_U: GhosttyKey = 40;
pub const GHOSTTY_KEY_V: GhosttyKey = 41;
pub const GHOSTTY_KEY_W: GhosttyKey = 42;
pub const GHOSTTY_KEY_X: GhosttyKey = 43;
pub const GHOSTTY_KEY_Y: GhosttyKey = 44;
pub const GHOSTTY_KEY_Z: GhosttyKey = 45;
pub const GHOSTTY_KEY_MINUS: GhosttyKey = 46;
pub const GHOSTTY_KEY_PERIOD: GhosttyKey = 47;
pub const GHOSTTY_KEY_QUOTE: GhosttyKey = 48;
pub const GHOSTTY_KEY_SEMICOLON: GhosttyKey = 49;
pub const GHOSTTY_KEY_SLASH: GhosttyKey = 50;
// Functional keys
pub const GHOSTTY_KEY_ALT_LEFT: GhosttyKey = 51;
pub const GHOSTTY_KEY_ALT_RIGHT: GhosttyKey = 52;
pub const GHOSTTY_KEY_BACKSPACE: GhosttyKey = 53;
pub const GHOSTTY_KEY_CAPS_LOCK: GhosttyKey = 54;
pub const GHOSTTY_KEY_CONTEXT_MENU: GhosttyKey = 55;
pub const GHOSTTY_KEY_CONTROL_LEFT: GhosttyKey = 56;
pub const GHOSTTY_KEY_CONTROL_RIGHT: GhosttyKey = 57;
pub const GHOSTTY_KEY_ENTER: GhosttyKey = 58;
pub const GHOSTTY_KEY_META_LEFT: GhosttyKey = 59;
pub const GHOSTTY_KEY_META_RIGHT: GhosttyKey = 60;
pub const GHOSTTY_KEY_SHIFT_LEFT: GhosttyKey = 61;
pub const GHOSTTY_KEY_SHIFT_RIGHT: GhosttyKey = 62;
pub const GHOSTTY_KEY_SPACE: GhosttyKey = 63;
pub const GHOSTTY_KEY_TAB: GhosttyKey = 64;
pub const GHOSTTY_KEY_CONVERT: GhosttyKey = 65;
pub const GHOSTTY_KEY_KANA_MODE: GhosttyKey = 66;
pub const GHOSTTY_KEY_NON_CONVERT: GhosttyKey = 67;
// Control pad
pub const GHOSTTY_KEY_DELETE: GhosttyKey = 68;
pub const GHOSTTY_KEY_END: GhosttyKey = 69;
pub const GHOSTTY_KEY_HELP: GhosttyKey = 70;
pub const GHOSTTY_KEY_HOME: GhosttyKey = 71;
pub const GHOSTTY_KEY_INSERT: GhosttyKey = 72;
pub const GHOSTTY_KEY_PAGE_DOWN: GhosttyKey = 73;
pub const GHOSTTY_KEY_PAGE_UP: GhosttyKey = 74;
// Arrow pad
pub const GHOSTTY_KEY_ARROW_DOWN: GhosttyKey = 75;
pub const GHOSTTY_KEY_ARROW_LEFT: GhosttyKey = 76;
pub const GHOSTTY_KEY_ARROW_RIGHT: GhosttyKey = 77;
pub const GHOSTTY_KEY_ARROW_UP: GhosttyKey = 78;
// Function keys
pub const GHOSTTY_KEY_ESCAPE: GhosttyKey = 118;
pub const GHOSTTY_KEY_F1: GhosttyKey = 119;
pub const GHOSTTY_KEY_F2: GhosttyKey = 120;
pub const GHOSTTY_KEY_F3: GhosttyKey = 121;
pub const GHOSTTY_KEY_F4: GhosttyKey = 122;
pub const GHOSTTY_KEY_F5: GhosttyKey = 123;
pub const GHOSTTY_KEY_F6: GhosttyKey = 124;
pub const GHOSTTY_KEY_F7: GhosttyKey = 125;
pub const GHOSTTY_KEY_F8: GhosttyKey = 126;
pub const GHOSTTY_KEY_F9: GhosttyKey = 127;
pub const GHOSTTY_KEY_F10: GhosttyKey = 128;
pub const GHOSTTY_KEY_F11: GhosttyKey = 129;
pub const GHOSTTY_KEY_F12: GhosttyKey = 130;

// Key encoder options
pub type GhosttyKeyEncoderOption = c_int;
pub const GHOSTTY_KEY_ENCODER_OPT_CURSOR_KEY_APPLICATION: GhosttyKeyEncoderOption = 0;
pub const GHOSTTY_KEY_ENCODER_OPT_KITTY_FLAGS: GhosttyKeyEncoderOption = 5;

// -------------------------------------------------------------------
// Focus encoding
// -------------------------------------------------------------------

pub type GhosttyFocusEvent = c_int;
pub const GHOSTTY_FOCUS_GAINED: GhosttyFocusEvent = 0;
pub const GHOSTTY_FOCUS_LOST: GhosttyFocusEvent = 1;

// -------------------------------------------------------------------
// Allocator (optional, NULL = default)
// -------------------------------------------------------------------

pub type GhosttyAllocator = c_void;

// -------------------------------------------------------------------
// Extern functions
// -------------------------------------------------------------------

extern "C" {
    // Terminal
    pub fn ghostty_terminal_new(
        allocator: *const GhosttyAllocator,
        terminal: *mut GhosttyTerminal,
        options: GhosttyTerminalOptions,
    ) -> GhosttyResult;
    pub fn ghostty_terminal_free(terminal: GhosttyTerminal);
    pub fn ghostty_terminal_reset(terminal: GhosttyTerminal);
    pub fn ghostty_terminal_resize(
        terminal: GhosttyTerminal,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> GhosttyResult;
    pub fn ghostty_terminal_set(
        terminal: GhosttyTerminal,
        option: GhosttyTerminalOption,
        value: *const c_void,
    ) -> GhosttyResult;
    pub fn ghostty_terminal_get(
        terminal: GhosttyTerminal,
        data: GhosttyTerminalData,
        out: *mut c_void,
    ) -> GhosttyResult;
    pub fn ghostty_terminal_vt_write(
        terminal: GhosttyTerminal,
        data: *const u8,
        len: usize,
    );
    pub fn ghostty_terminal_scroll_viewport(
        terminal: GhosttyTerminal,
        behavior: GhosttyTerminalScrollViewport,
    );
    pub fn ghostty_terminal_mode_get(
        terminal: GhosttyTerminal,
        mode: c_int,
        out_value: *mut bool,
    ) -> GhosttyResult;
    pub fn ghostty_terminal_mode_set(
        terminal: GhosttyTerminal,
        mode: c_int,
        value: bool,
    ) -> GhosttyResult;

    // Render state
    pub fn ghostty_render_state_new(
        allocator: *const GhosttyAllocator,
        state: *mut GhosttyRenderState,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_free(state: GhosttyRenderState);
    pub fn ghostty_render_state_update(
        state: GhosttyRenderState,
        terminal: GhosttyTerminal,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_get(
        state: GhosttyRenderState,
        data: GhosttyRenderStateData,
        out: *mut c_void,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_set(
        state: GhosttyRenderState,
        option: GhosttyRenderStateOption,
        value: *const c_void,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_colors_get(
        state: GhosttyRenderState,
        out_colors: *mut GhosttyRenderStateColors,
    ) -> GhosttyResult;

    // Row iterator
    pub fn ghostty_render_state_row_iterator_new(
        allocator: *const GhosttyAllocator,
        out_iterator: *mut GhosttyRenderStateRowIterator,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_row_iterator_free(iterator: GhosttyRenderStateRowIterator);
    pub fn ghostty_render_state_row_iterator_next(
        iterator: GhosttyRenderStateRowIterator,
    ) -> bool;
    pub fn ghostty_render_state_row_get(
        iterator: GhosttyRenderStateRowIterator,
        data: GhosttyRenderStateRowData,
        out: *mut c_void,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_row_set(
        iterator: GhosttyRenderStateRowIterator,
        option: GhosttyRenderStateRowOption,
        value: *const c_void,
    ) -> GhosttyResult;

    // Row cells
    pub fn ghostty_render_state_row_cells_new(
        allocator: *const GhosttyAllocator,
        out_cells: *mut GhosttyRenderStateRowCells,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_row_cells_next(cells: GhosttyRenderStateRowCells) -> bool;
    pub fn ghostty_render_state_row_cells_select(
        cells: GhosttyRenderStateRowCells,
        x: u16,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_row_cells_get(
        cells: GhosttyRenderStateRowCells,
        data: GhosttyRenderStateRowCellsData,
        out: *mut c_void,
    ) -> GhosttyResult;
    pub fn ghostty_render_state_row_cells_free(cells: GhosttyRenderStateRowCells);

    // Key event
    pub fn ghostty_key_event_new(
        allocator: *const GhosttyAllocator,
        event: *mut GhosttyKeyEvent,
    ) -> GhosttyResult;
    pub fn ghostty_key_event_free(event: GhosttyKeyEvent);
    pub fn ghostty_key_event_set_action(event: GhosttyKeyEvent, action: GhosttyKeyAction);
    pub fn ghostty_key_event_set_key(event: GhosttyKeyEvent, key: GhosttyKey);
    pub fn ghostty_key_event_set_mods(event: GhosttyKeyEvent, mods: GhosttyMods);
    pub fn ghostty_key_event_set_consumed_mods(event: GhosttyKeyEvent, consumed_mods: GhosttyMods);
    pub fn ghostty_key_event_set_composing(event: GhosttyKeyEvent, composing: bool);
    pub fn ghostty_key_event_set_utf8(event: GhosttyKeyEvent, utf8: *const c_char, len: usize);
    pub fn ghostty_key_event_set_unshifted_codepoint(event: GhosttyKeyEvent, codepoint: u32);

    // Key encoder
    pub fn ghostty_key_encoder_new(
        allocator: *const GhosttyAllocator,
        encoder: *mut GhosttyKeyEncoder,
    ) -> GhosttyResult;
    pub fn ghostty_key_encoder_free(encoder: GhosttyKeyEncoder);
    pub fn ghostty_key_encoder_setopt(
        encoder: GhosttyKeyEncoder,
        option: GhosttyKeyEncoderOption,
        value: *const c_void,
    );
    pub fn ghostty_key_encoder_setopt_from_terminal(
        encoder: GhosttyKeyEncoder,
        terminal: GhosttyTerminal,
    );
    pub fn ghostty_key_encoder_encode(
        encoder: GhosttyKeyEncoder,
        event: GhosttyKeyEvent,
        out_buf: *mut c_char,
        out_buf_size: usize,
        out_len: *mut usize,
    ) -> GhosttyResult;

    // Focus
    pub fn ghostty_focus_encode(
        event: GhosttyFocusEvent,
        buf: *mut c_char,
        buf_len: usize,
        out_written: *mut usize,
    ) -> GhosttyResult;

    // Style
    pub fn ghostty_style_default(style: *mut GhosttyStyle);
    pub fn ghostty_style_is_default(style: *const GhosttyStyle) -> bool;

    // Color
    pub fn ghostty_color_rgb_get(color: GhosttyColorRgb, r: *mut u8, g: *mut u8, b: *mut u8);
}
