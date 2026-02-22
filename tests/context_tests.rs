use xpresso::{Action, Context, Mode, Rule};

// --- Mode ---

#[test]
fn mode_equality() {
    assert_eq!(Mode::Echo, Mode::Echo);
    assert_eq!(Mode::Firewall, Mode::Firewall);
    assert_ne!(Mode::Echo, Mode::Firewall);
}

#[test]
fn mode_is_copy() {
    let m = Mode::Echo;
    let m2 = m;
    assert_eq!(m, m2);
}

#[test]
fn mode_is_clone() {
    let m = Mode::Firewall;
    let m2 = m.clone();
    assert_eq!(m, m2);
}

#[test]
fn mode_debug_is_non_empty() {
    assert!(!format!("{:?}", Mode::Echo).is_empty());
    assert!(!format!("{:?}", Mode::Firewall).is_empty());
}

#[test]
fn action_equality() {
    assert_eq!(Action::Pass, Action::Pass);
    assert_eq!(Action::Drop, Action::Drop);
    assert_ne!(Action::Pass, Action::Drop);
}

#[test]
fn action_is_copy() {
    let a = Action::Pass;
    let a2 = a;
    assert_eq!(a, a2);
}

#[test]
fn action_is_clone() {
    let a = Action::Drop;
    let a2 = a.clone();
    assert_eq!(a, a2);
}

// --- Rule ---

#[test]
fn rule_deny_port_creates_correct_variant() {
    let rule = Rule::deny_port(8080);
    assert!(matches!(rule, Rule::DenyPort(8080)));
}

#[test]
fn rule_deny_port_zero() {
    let rule = Rule::deny_port(0);
    assert!(matches!(rule, Rule::DenyPort(0)));
}

#[test]
fn rule_deny_port_max() {
    let rule = Rule::deny_port(u16::MAX);
    assert!(matches!(rule, Rule::DenyPort(p) if p == u16::MAX));
}

#[test]
fn rule_allow_cidr_creates_correct_variant() {
    let rule = Rule::allow_cidr("192.168.1.0/24");
    assert!(matches!(rule, Rule::AllowCidr(ref s) if s == "192.168.1.0/24"));
}

#[test]
fn rule_allow_cidr_stores_string_as_given() {
    let cidr = "10.0.0.0/8";
    let rule = Rule::allow_cidr(cidr);
    match rule {
        Rule::AllowCidr(s) => assert_eq!(s, cidr),
        _ => panic!("expected AllowCidr"),
    }
}

#[test]
fn rule_is_clone() {
    let r = Rule::deny_port(443);
    let r2 = r.clone();
    assert!(matches!(r2, Rule::DenyPort(443)));

    let r3 = Rule::allow_cidr("10.0.0.0/8");
    let r4 = r3.clone();
    assert!(matches!(r4, Rule::AllowCidr(ref s) if s == "10.0.0.0/8"));
}

#[test]
fn context_new_echo_mode() {
    // Just verify construction doesn't panic
    let _ctx = Context::new(Mode::Echo);
}

#[test]
fn context_new_firewall_mode() {
    let _ctx = Context::new(Mode::Firewall);
}

#[test]
fn context_set_default_action_does_not_panic() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_default_action(Action::Drop);
    ctx.set_default_action(Action::Pass);
}

#[test]
fn context_set_port_does_not_panic() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_port(0);
    ctx.set_port(8080);
    ctx.set_port(u16::MAX);
}

#[test]
fn context_set_iface_does_not_panic() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_iface("eth0");
    ctx.set_iface("lo");
}

#[test]
fn context_set_skb_mode_does_not_panic() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.set_skb_mode(true);
    ctx.set_skb_mode(false);
}

#[test]
fn context_add_multiple_rules_does_not_panic() {
    let mut ctx = Context::new(Mode::Firewall);
    ctx.add_rule(Rule::deny_port(22));
    ctx.add_rule(Rule::deny_port(80));
    ctx.add_rule(Rule::allow_cidr("10.0.0.0/8"));
    ctx.add_rule(Rule::allow_cidr("192.168.0.0/16"));
}
