use std::io;
use xpresso::FlightError;

#[test]
fn display_bind_error() {
    let err = FlightError::Bind("0.0.0.0:8080: address in use".to_string());
    assert_eq!(err.to_string(), "bind failed: 0.0.0.0:8080: address in use");
}

#[test]
fn display_send_error() {
    let err = FlightError::Send("connection refused".to_string());
    assert_eq!(err.to_string(), "send failed: connection refused");
}

#[test]
fn display_recv_error() {
    let err = FlightError::Recv("timed out".to_string());
    assert_eq!(err.to_string(), "recv failed: timed out");
}

#[test]
fn display_not_attached() {
    let err = FlightError::NotAttached;
    assert_eq!(err.to_string(), "program not attached, call attach() first");
}

#[test]
fn display_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err = FlightError::Io(io_err);
    assert!(err.to_string().starts_with("io error:"));
}

#[test]
fn from_io_error_produces_io_variant() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let err: FlightError = io_err.into();
    assert!(matches!(err, FlightError::Io(_)));
}

#[test]
fn implements_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(FlightError::NotAttached);
    assert!(!err.to_string().is_empty());
}

#[test]
fn debug_format_is_non_empty() {
    let err = FlightError::Bind("test".to_string());
    assert!(!format!("{err:?}").is_empty());
}
