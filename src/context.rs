#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Echo,
    Firewall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Pass,
    Drop,
}

#[derive(Debug, Clone)]
pub enum Rule {
    DenyPort(u16),
    AllowCidr(String),
}

impl Rule {
    pub fn deny_port(port: u16) -> Self {
        Self::DenyPort(port)
    }

    pub fn allow_cidr(cidr: &str) -> Self {
        Self::AllowCidr(cidr.to_string())
    }
}

pub struct Context {
    pub(crate) mode: Mode,
    pub(crate) default_action: Action,
    pub(crate) rules: Vec<Rule>,
    pub(crate) port: u16,
    pub(crate) iface: Option<String>,
}

impl Context {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            default_action: Action::Pass,
            rules: Vec::new(),
            port: 0,
            iface: None,
        }
    }

    pub fn set_default_action(&mut self, action: Action) {
        self.default_action = action;
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn set_iface(&mut self, iface: &str) {
        self.iface = Some(iface.to_string());
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub(crate) fn to_platform_config(&self) -> crate::platform::PlatformConfig {
        crate::platform::PlatformConfig {
            port: self.port,
            iface: self.iface.clone(),
        }
    }
}
