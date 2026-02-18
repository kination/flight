use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")] // beautiful serialization
pub enum Mode {
    Echo,
    Firewall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Pass,
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")] // cleaner enum serialization
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub(crate) mode: Mode,
    #[serde(default = "default_action")]
    pub(crate) default_action: Action,
    #[serde(default)]
    pub(crate) rules: Vec<Rule>,
    #[serde(default)]
    pub(crate) port: u16,
    #[serde(default)]
    pub(crate) iface: Option<String>,
    #[serde(default = "default_skb_mode")]
    pub(crate) skb_mode: bool,
    #[serde(default)]
    pub(crate) bind_ip: Option<String>,
}

fn default_action() -> Action {
    Action::Pass
}

fn default_skb_mode() -> bool {
    true
}

impl Context {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            default_action: Action::Pass,
            rules: Vec::new(),
            port: 0,
            iface: None,
            skb_mode: true,
            bind_ip: None,
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

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn iface(&self) -> Option<&str> {
        self.iface.as_deref()
    }

    pub fn bind_ip(&self) -> Option<&str> {
        self.bind_ip.as_deref()
    }

    pub fn set_skb_mode(&mut self, skb_mode: bool) {
        self.skb_mode = skb_mode;
    }

    pub fn set_bind_ip(&mut self, ip: &str) {
        self.bind_ip = Some(ip.to_string());
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub(crate) fn to_platform_config(&self) -> crate::platform::PlatformConfig {
        crate::platform::PlatformConfig {
            port: self.port,
            iface: self.iface.clone(),
            skb_mode: self.skb_mode,
            bind_ip: self.bind_ip.clone(),
        }
    }
}
