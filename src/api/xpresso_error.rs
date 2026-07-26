use std::fmt;

#[derive(Debug)]
pub enum XpressoError {
    InvalidConfig(String),
    Unimplemented,
}

impl fmt::Display for XpressoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(m) => write!(f, "Invalid config: {}", m),
            Self::Unimplemented => write!(f, "Unimplemented"),
        }
    }
}

impl std::error::Error for XpressoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
