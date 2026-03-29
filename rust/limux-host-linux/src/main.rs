mod app_config;
mod keybind_editor;
mod layout_state;
mod pane;
mod pty;
mod shortcut_config;
mod terminal;
mod vt_terminal;
mod window;

use adw::prelude::*;
use libadwaita as adw;
use std::path::{Path, PathBuf};

const APP_ID: &str = "dev.limux.linux";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Append a value to an environment variable (comma-separated), or set it.
fn append_env(key: &str, value: &str) {
    match std::env::var(key) {
        Ok(existing) if !existing.is_empty() => {
            std::env::set_var(key, format!("{existing},{value}"));
        }
        _ => {
            std::env::set_var(key, value);
        }
    }
}

fn has_ghostty_terminfo(path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };

    ["terminfo/g/ghostty", "terminfo/x/xterm-ghostty"]
        .iter()
        .any(|entry| parent.join(entry).is_file())
}

fn is_ghostty_resources_dir(path: &Path) -> bool {
    path.is_dir()
        && ["themes", "shell-integration"]
            .iter()
            .all(|entry| path.join(entry).is_dir())
        && has_ghostty_terminfo(path)
}

fn ghostty_resources_candidates(exe_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for ancestor in exe_dir.ancestors() {
        candidates.push(ancestor.join("share/limux/ghostty"));
        candidates.push(ancestor.join("share/ghostty"));
        candidates.push(ancestor.join("ghostty/zig-out/share/ghostty"));
    }

    candidates.push(PathBuf::from("/usr/local/share/ghostty"));
    candidates.push(PathBuf::from("/usr/share/ghostty"));

    candidates
}

fn resolve_ghostty_resources_dir(exe_path: &Path) -> Option<PathBuf> {
    let exe_dir = exe_path.parent()?;
    ghostty_resources_candidates(exe_dir)
        .into_iter()
        .find(|path| is_ghostty_resources_dir(path))
}

fn set_ghostty_resources_env() {
    if std::env::var_os("GHOSTTY_RESOURCES_DIR").is_some() {
        return;
    }

    let Some(exe_path) = std::env::current_exe().ok() else {
        return;
    };

    if let Some(path) = resolve_ghostty_resources_dir(&exe_path) {
        std::env::set_var("GHOSTTY_RESOURCES_DIR", path);
    }
}

fn suppress_gtk_theme_warnings() {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_uint, c_void};

    // GLib log handler signature
    type GLogFunc = unsafe extern "C" fn(*const c_char, c_uint, *const c_char, *mut c_void);

    extern "C" {
        fn g_log_set_handler(
            log_domain: *const c_char,
            log_levels: c_uint,
            log_func: GLogFunc,
            user_data: *mut c_void,
        ) -> c_uint;
    }

    unsafe extern "C" fn filter_handler(
        domain: *const c_char,
        level: c_uint,
        message: *const c_char,
        _user_data: *mut c_void,
    ) {
        let msg = if message.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(message).to_str().unwrap_or("")
        };
        // Suppress "Theme parser error" warnings from missing CSS resources
        if msg.contains("Theme parser error") {
            return;
        }
        let dom = if domain.is_null() {
            "Gtk"
        } else {
            std::ffi::CStr::from_ptr(domain).to_str().unwrap_or("Gtk")
        };
        // G_LOG_LEVEL_WARNING = 1 << 4 = 16
        if level & 16 != 0 {
            eprintln!("({dom}): WARNING: {msg}");
        } else {
            eprintln!("({dom}): {msg}");
        }
    }

    let domain = CString::new("Gtk").unwrap();
    // G_LOG_LEVEL_WARNING | G_LOG_LEVEL_CRITICAL = (1<<4) | (1<<3) = 0x18
    const G_LOG_LEVEL_WARNING: c_uint = 1 << 4;
    unsafe {
        g_log_set_handler(
            domain.as_ptr(),
            G_LOG_LEVEL_WARNING,
            filter_handler,
            std::ptr::null_mut(),
        );
    }
}

fn main() {
    // Handle --version flag
    if std::env::args().any(|a| a == "--version" || a == "-v") {
        println!("Limux {VERSION}");
        return;
    }

    // Ghostty resources directory for themes and shell integration.
    set_ghostty_resources_env();

    // WebKitGTK's bubblewrap sandbox requires unprivileged user namespaces,
    // which may not be available. Disable it to prevent crashes on launch.
    if std::env::var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS").is_err() {
        std::env::set_var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");
    }

    // Suppress GTK CSS "Theme parser error" warnings caused by missing system
    // theme resources (e.g. Arc theme not installed). These are non-fatal and
    // clutter stderr, and on some systems can cascade into crashes.
    suppress_gtk_theme_warnings();

    // VT-based terminal: no global ghostty app initialization needed.
    vt_terminal::init_ghostty();

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(adw::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        window::build_window(app);
    });
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("limux-{label}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn resolves_app_specific_bundled_resources_next_to_executable() {
        let root = temp_path("resources");
        let exe_dir = root.join("bin");
        let themes_dir = root.join("share/limux/ghostty/themes");
        let shell_integration_dir = root.join("share/limux/ghostty/shell-integration");
        let terminfo_file = root.join("share/limux/terminfo/g/ghostty");
        fs::create_dir_all(&exe_dir).unwrap();
        fs::create_dir_all(&themes_dir).unwrap();
        fs::create_dir_all(&shell_integration_dir).unwrap();
        fs::create_dir_all(terminfo_file.parent().unwrap()).unwrap();
        fs::write(&terminfo_file, b"ghostty").unwrap();

        let exe = exe_dir.join("limux");
        let resolved = resolve_ghostty_resources_dir(&exe).unwrap();
        assert_eq!(resolved, root.join("share/limux/ghostty"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_dev_checkout_resources_from_target_binary() {
        let root = temp_path("dev-resources");
        let exe_dir = root.join("target/release");
        let themes_dir = root.join("ghostty/zig-out/share/ghostty/themes");
        let shell_integration_dir = root.join("ghostty/zig-out/share/ghostty/shell-integration");
        let terminfo_file = root.join("ghostty/zig-out/share/terminfo/x/xterm-ghostty");
        fs::create_dir_all(&exe_dir).unwrap();
        fs::create_dir_all(&themes_dir).unwrap();
        fs::create_dir_all(&shell_integration_dir).unwrap();
        fs::create_dir_all(terminfo_file.parent().unwrap()).unwrap();
        fs::write(&terminfo_file, b"xterm-ghostty").unwrap();

        let exe = exe_dir.join("limux");
        let resolved = resolve_ghostty_resources_dir(&exe).unwrap();
        assert_eq!(resolved, root.join("ghostty/zig-out/share/ghostty"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_resource_dirs_without_sibling_terminfo() {
        let root = temp_path("missing-terminfo");
        let resources_dir = root.join("ghostty/zig-out/share/ghostty");
        let themes_dir = resources_dir.join("themes");
        let shell_integration_dir = resources_dir.join("shell-integration");
        fs::create_dir_all(&themes_dir).unwrap();
        fs::create_dir_all(&shell_integration_dir).unwrap();

        assert!(!is_ghostty_resources_dir(&resources_dir));

        fs::remove_dir_all(root).unwrap();
    }
}
