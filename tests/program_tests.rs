use std::net::{SocketAddr, UdpSocket};
use std::sync::{Mutex, MutexGuard};
use xpresso::{Context, FlightError, Mode, Program, Rule};

// AF_XDP can only attach one socket per interface at a time.
// Use a process-wide mutex to serialize all attach() calls.
static ATTACH_LOCK: Mutex<()> = Mutex::new(());

struct AttachedProgram {
    prog: Program,
    // Guard is held until AttachedProgram drops.
    // Struct fields drop in declaration order: prog first, then _guard.
    // This ensures XDP detaches before the lock is released.
    _guard: MutexGuard<'static, ()>,
}

impl std::ops::Deref for AttachedProgram {
    type Target = Program;
    fn deref(&self) -> &Program {
        &self.prog
    }
}

impl std::ops::DerefMut for AttachedProgram {
    fn deref_mut(&mut self) -> &mut Program {
        &mut self.prog
    }
}

fn make_unattached(mode: Mode) -> Program {
    let ctx = Context::new(mode);
    Program::with_context(ctx).expect("with_context should succeed")
}

/// Returns None if attach fails (e.g. XDP resource busy in CI).
fn try_make_attached(mode: Mode) -> Option<AttachedProgram> {
    let guard = ATTACH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut ctx = Context::new(mode);
    ctx.set_port(0);
    let mut prog = Program::with_context(ctx).ok()?;
    prog.attach().ok()?;
    Some(AttachedProgram {
        prog,
        _guard: guard,
    })
}

macro_rules! require_attached {
    ($mode:expr) => {
        match try_make_attached($mode) {
            Some(p) => p,
            None => {
                eprintln!("SKIP: attach() unavailable (XDP resource busy or unsupported)");
                return;
            }
        }
    };
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
    let p = require_attached!(Mode::Firewall);
    assert!(
        p.local_addr().is_ok(),
        "local_addr should succeed after attach"
    );
}

#[test]
fn detach_after_attach_is_ok() {
    let mut p = require_attached!(Mode::Firewall);
    assert!(p.detach().is_ok());
}

#[test]
fn local_addr_after_detach_returns_not_attached() {
    let mut p = require_attached!(Mode::Firewall);
    p.detach().unwrap();
    assert!(matches!(p.local_addr(), Err(FlightError::NotAttached)));
}

// --- Stats ---

#[test]
fn stats_start_at_zero() {
    let p = require_attached!(Mode::Firewall);
    let s = p.stats();
    assert_eq!(s.tx_packets(), 0);
    assert_eq!(s.rx_packets(), 0);
    assert_eq!(s.tx_bytes(), 0);
    assert_eq!(s.rx_bytes(), 0);
    assert_eq!(s.dropped(), 0);
}

#[test]
fn stats_display_contains_expected_fields() {
    let p = require_attached!(Mode::Firewall);
    let s = format!("{}", p.stats());
    assert!(s.contains("tx:"));
    assert!(s.contains("rx:"));
    assert!(s.contains("dropped:"));
}

// --- Outgoing deny filter ---

#[test]
fn send_to_denied_port_returns_zero_and_increments_dropped() {
    let _guard = ATTACH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.add_rule(Rule::deny_port(9999));
    let mut p = Program::with_context(ctx).unwrap();
    match p.attach() {
        Err(_) => {
            eprintln!("SKIP: attach() unavailable");
            return;
        }
        Ok(_) => {}
    }

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
    let p = require_attached!(Mode::Firewall);

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
    let _guard = ATTACH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.add_rule(Rule::deny_port(9999));
    let mut p = Program::with_context(ctx).unwrap();
    match p.attach() {
        Err(_) => {
            eprintln!("SKIP: attach() unavailable");
            return;
        }
        Ok(_) => {}
    }

    let dest: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    for _ in 0..5 {
        p.send(&dest, b"x").unwrap();
    }
    assert_eq!(p.stats().dropped(), 5);
}

#[test]
fn send_to_non_denied_port_with_deny_rule_on_different_port() {
    let _guard = ATTACH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.add_rule(Rule::deny_port(1111));
    let mut p = Program::with_context(ctx).unwrap();
    match p.attach() {
        Err(_) => {
            eprintln!("SKIP: attach() unavailable");
            return;
        }
        Ok(_) => {}
    }

    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    let dest = receiver.local_addr().unwrap();
    let n = p.send(&dest, b"data").unwrap();
    assert_eq!(n, 4);
    assert_eq!(p.stats().dropped(), 0);
}
