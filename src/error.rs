use std::fmt;

#[derive(Debug)]
pub enum FlightError {
    Bind(String),
    Send(String),
    Recv(String),
    NotAttached,
    Io(std::io::Error),
}

impl fmt::Display for FlightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(msg) => write!(f, "bind failed: {msg}"),
            Self::Send(msg) => write!(f, "send failed: {msg}"),
            Self::Recv(msg) => write!(f, "recv failed: {msg}"),
            Self::NotAttached => write!(f, "program not attached, call attach() first"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for FlightError {}

impl From<std::io::Error> for FlightError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
