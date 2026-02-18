use std::net::{SocketAddr, UdpSocket};
use xpresso::{Context, FlightError, Mode, Program, Rule};

fn make_unattached(mode: Mode) -> Program {
    let ctx = Context::new(mode);
    Program::with_context(ctx).expect("with_context should succeed")
}

fn make_attached(mode: Mode) -> Program {
    let mut ctx = Context::new(mode);
    ctx.set_port(0); // ephemeral port
    let mut p = Program::with_context(ctx).unwrap();
    p.attach().expect("attach should succeed");
    p
}

// --- Construction ---

#[test]
fn with_context_succeeds_for_echo() {
    let ctx = Context::new(Mode::Echo);
    assert!(Program::with_context(ctx).is_ok());
}

#[test]
fn with_context_succeeds_for_firewall() {
    let ctx = Context::new(Mode::Firewall);
    assert!(Program::with_context(ctx).is_ok());
}

// --- Unattached errors ---

#[test]
fn send_without_attach_returns_not_attached() {
    let p = make_unattached(Mode::Firewall);
    let dest: SocketAddr = "127.0.0.1:9000".parse().unwrap();
    let result = p.send(&dest, b"hello");
    assert!(matches!(result, Err(FlightError::NotAttached)));
}

#[test]
fn recv_without_attach_returns_not_attached() {
    let p = make_unattached(Mode::Firewall);
    let mut buf = [0u8; 128];
    let result = p.recv(&mut buf);
    assert!(matches!(result, Err(FlightError::NotAttached)));
}

#[test]
fn local_addr_without_attach_returns_not_attached() {
    let p = make_unattached(Mode::Firewall);
    let result = p.local_addr();
    assert!(matches!(result, Err(FlightError::NotAttached)));
}

// --- Attach / detach lifecycle ---

#[test]
fn detach_without_attach_is_ok() {
    let mut p = make_unattached(Mode::Firewall);
    assert!(p.detach().is_ok());
}

#[test]
fn attach_succeeds_and_local_addr_is_reachable() {
    let p = make_attached(Mode::Firewall);
    let addr = p
        .local_addr()
        .expect("local_addr after attach should succeed");
    assert_eq!(addr.ip().to_string(), "0.0.0.0");
    assert_ne!(
        addr.port(),
        0,
        "ephemeral port should be non-zero after bind"
    );
}

#[test]
fn detach_after_attach_is_ok() {
    let mut p = make_attached(Mode::Firewall);
    assert!(p.detach().is_ok());
}

#[test]
fn local_addr_after_detach_returns_not_attached() {
    let mut p = make_attached(Mode::Firewall);
    p.detach().unwrap();
    assert!(matches!(p.local_addr(), Err(FlightError::NotAttached)));
}

// --- Stats ---

#[test]
fn stats_start_at_zero() {
    let p = make_attached(Mode::Firewall);
    let s = p.stats();
    assert_eq!(s.tx_packets(), 0);
    assert_eq!(s.rx_packets(), 0);
    assert_eq!(s.tx_bytes(), 0);
    assert_eq!(s.rx_bytes(), 0);
    assert_eq!(s.dropped(), 0);
}

#[test]
fn stats_display_contains_expected_fields() {
    let p = make_attached(Mode::Firewall);
    let s = format!("{}", p.stats());
    assert!(s.contains("tx:"));
    assert!(s.contains("rx:"));
    assert!(s.contains("dropped:"));
}

// --- Outgoing deny filter ---

#[test]
fn send_to_denied_port_returns_zero_and_increments_dropped() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.add_rule(Rule::deny_port(9999));
    let mut p = Program::with_context(ctx).unwrap();
    p.attach().unwrap();

    let dest: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    let n = p
        .send(&dest, b"hello")
        .expect("send should not error for denied");
    assert_eq!(n, 0, "denied send should return 0 bytes");
    assert_eq!(p.stats().dropped(), 1);
    assert_eq!(p.stats().tx_packets(), 0);
    assert_eq!(p.stats().tx_bytes(), 0);
}

#[test]
fn send_to_non_denied_port_succeeds_and_updates_tx_stats() {
    let p = make_attached(Mode::Firewall);

    // Bind a receiver so we have a valid destination
    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    let dest = receiver.local_addr().unwrap();

    let data = b"hello";
    let n = p.send(&dest, data).expect("send should succeed");
    assert_eq!(n, data.len());
    assert_eq!(p.stats().tx_packets(), 1);
    assert_eq!(p.stats().tx_bytes(), data.len() as u64);
    assert_eq!(p.stats().dropped(), 0);
}

#[test]
fn multiple_denied_sends_accumulate_dropped_count() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.add_rule(Rule::deny_port(9999));
    let mut p = Program::with_context(ctx).unwrap();
    p.attach().unwrap();

    let dest: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    for _ in 0..5 {
        p.send(&dest, b"x").unwrap();
    }
    assert_eq!(p.stats().dropped(), 5);
}

#[test]
fn send_to_non_denied_port_with_deny_rule_on_different_port() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.add_rule(Rule::deny_port(1111)); // deny 1111, not 2222
    let mut p = Program::with_context(ctx).unwrap();
    p.attach().unwrap();

    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    let dest = receiver.local_addr().unwrap();
    // dest port is ephemeral (not 1111), so should pass
    let n = p.send(&dest, b"data").unwrap();
    assert_eq!(n, 4);
    assert_eq!(p.stats().dropped(), 0);
}
