use std::fmt;

pub enum SystemOs {
    Win32,
    Linux,
    MacOS,
    UnixLike,
    Unknown,
}

pub const fn get_os() -> SystemOs {
    if cfg!(windows) {
        SystemOs::Win32
    } else if cfg!(target_os = "linux") {
        SystemOs::Linux
    } else if cfg!(target_os = "macos") {
        SystemOs::MacOS
    } else if cfg!(unix) {
        SystemOs::UnixLike
    } else {
        SystemOs::Unknown
    }
}

pub const fn is_debug() -> bool {
    cfg!(debug_assertions)
}

impl fmt::Display for SystemOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Win32 => "Windows NT",
            Self::Linux => "Linux",
            Self::MacOS => "macOS",
            Self::UnixLike => "Unix-like",
            Self::Unknown => "Unknown",
        })
    }
}
