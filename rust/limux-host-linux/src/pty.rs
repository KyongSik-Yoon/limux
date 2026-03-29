use std::ffi::CString;
use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

/// A PTY master handle with a spawned child process.
pub struct PtyHandle {
    master: OwnedFd,
    child_pid: rustix::process::Pid,
}

impl PtyHandle {
    /// Open a PTY, fork, and exec the user's shell in the child.
    pub fn spawn(
        shell: &str,
        working_dir: Option<&str>,
        cols: u16,
        rows: u16,
        cell_width_px: u16,
        cell_height_px: u16,
    ) -> io::Result<Self> {
        // Open a new PTY pair
        let master = rustix::pty::openpt(rustix::pty::OpenptFlags::RDWR | rustix::pty::OpenptFlags::NOCTTY)
            .map_err(io::Error::from)?;
        rustix::pty::grantpt(&master).map_err(io::Error::from)?;
        rustix::pty::unlockpt(&master).map_err(io::Error::from)?;

        let slave_name_buf = vec![0u8; 256];
        let slave_path = rustix::pty::ptsname(&master, slave_name_buf)
            .map_err(io::Error::from)?;

        // Set window size before fork
        set_winsize(master.as_fd(), cols, rows, cell_width_px, cell_height_px)?;

        // Fork
        let fork_result = unsafe { libc::fork() };
        if fork_result < 0 {
            return Err(io::Error::last_os_error());
        }

        if fork_result == 0 {
            // === Child process ===
            drop(master);

            // New session
            unsafe { libc::setsid() };

            // Open slave as controlling terminal
            let slave_fd = unsafe {
                libc::open(slave_path.as_ptr(), libc::O_RDWR)
            };
            if slave_fd < 0 {
                unsafe { libc::_exit(1) };
            }

            unsafe {
                libc::ioctl(slave_fd, libc::TIOCSCTTY, 0);
                libc::dup2(slave_fd, 0);
                libc::dup2(slave_fd, 1);
                libc::dup2(slave_fd, 2);
                if slave_fd > 2 {
                    libc::close(slave_fd);
                }
            }

            // Set up environment
            let term = CString::new("xterm-256color").unwrap();
            let colorterm = CString::new("truecolor").unwrap();
            let term_key = CString::new("TERM").unwrap();
            let colorterm_key = CString::new("COLORTERM").unwrap();
            unsafe {
                libc::setenv(term_key.as_ptr(), term.as_ptr(), 1);
                libc::setenv(colorterm_key.as_ptr(), colorterm.as_ptr(), 1);
            }

            // Change working directory
            if let Some(wd) = working_dir {
                if let Ok(cwd) = CString::new(wd) {
                    unsafe { libc::chdir(cwd.as_ptr()) };
                }
            }

            // Exec the shell
            let shell_cstr = CString::new(shell).unwrap_or_else(|_| {
                CString::new("/bin/sh").unwrap()
            });
            let login_arg = CString::new("-l").unwrap();
            let argv = [shell_cstr.as_ptr(), login_arg.as_ptr(), std::ptr::null()];
            unsafe {
                libc::execvp(shell_cstr.as_ptr(), argv.as_ptr());
                libc::_exit(127);
            }
        }

        // === Parent process ===
        // Set master to non-blocking
        let flags = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_GETFL) };
        unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) };

        let child_pid = unsafe { rustix::process::Pid::from_raw_unchecked(fork_result) };

        Ok(PtyHandle { master, child_pid })
    }

    pub fn master_fd(&self) -> BorrowedFd<'_> {
        self.master.as_fd()
    }

    pub fn raw_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Read available data from the PTY master (non-blocking).
    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe {
            libc::read(self.master.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len())
        };
        if n < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    /// Write data to the PTY master.
    pub fn write(&self, data: &[u8]) -> io::Result<usize> {
        let n = unsafe {
            libc::write(self.master.as_raw_fd(), data.as_ptr().cast(), data.len())
        };
        if n < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    /// Resize the PTY.
    pub fn resize(
        &self,
        cols: u16,
        rows: u16,
        cell_width_px: u16,
        cell_height_px: u16,
    ) -> io::Result<()> {
        set_winsize(self.master.as_fd(), cols, rows, cell_width_px, cell_height_px)
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        // Send SIGHUP to the child process group
        unsafe {
            libc::kill(-(self.child_pid.as_raw_nonzero().get()), libc::SIGHUP);
        }
    }
}

fn set_winsize(
    fd: BorrowedFd<'_>,
    cols: u16,
    rows: u16,
    xpixel: u16,
    ypixel: u16,
) -> io::Result<()> {
    let ws = libc::winsize {
        ws_col: cols,
        ws_row: rows,
        ws_xpixel: xpixel,
        ws_ypixel: ypixel,
    };
    let ret = unsafe { libc::ioctl(fd.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Get the user's default shell from /etc/passwd or $SHELL.
pub fn default_shell() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.is_empty() {
            return shell;
        }
    }
    // Fallback: read from passwd
    unsafe {
        let uid = libc::getuid();
        let pw = libc::getpwuid(uid);
        if !pw.is_null() {
            let shell = std::ffi::CStr::from_ptr((*pw).pw_shell);
            if let Ok(s) = shell.to_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }
    "/bin/sh".to_string()
}
