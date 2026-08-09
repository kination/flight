use std::fmt;

#[derive(Debug)]
pub enum XpressoError {
    InvalidConfig(String),
    PlatformNotSupported(String),
    Io(std::io::Error),
    Unimplemented,
}

impl fmt::Display for XpressoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            Self::PlatformNotSupported(msg) => write!(f, "Platform not supported: {}", msg),
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Unimplemented => write!(f, "Unimplemented"),
        }
    }
}

impl std::error::Error for XpressoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<std::io::Error> for XpressoError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
