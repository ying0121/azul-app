#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    /// Default when the user opens the app — show the main window.
    Standard,
    /// Background launch (e.g. startup shortcut) — tray icon only, no window.
    TrayOnly,
}

impl LaunchMode {
    pub fn from_env_args() -> Self {
        let tray_flag = std::env::args().any(|arg| {
            matches!(
                arg.as_str(),
                "--tray" | "-tray" | "/tray" | "--background" | "-background"
            )
        });
        if tray_flag {
            Self::TrayOnly
        } else {
            Self::Standard
        }
    }
}
