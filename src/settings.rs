use crate::rpc::RpcConnectionSettings;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub daemon_ip: String,
    pub daemon_port: u16,
    pub daemon_transport: String,
    pub daemon_restricted_mode: bool,
    pub daemon_login_enabled: bool,
    pub daemon_login_username: String,
    pub daemon_login_password: String,
    pub wallet_rpc_enabled: bool,
    pub wallet_ip: String,
    pub wallet_port: u16,
    pub wallet_transport: String,
    pub wallet_login_enabled: bool,
    pub wallet_login_username: String,
    pub wallet_login_password: String,
    pub poll_frequency_seconds: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            daemon_ip: "127.0.0.1".to_string(),
            daemon_port: 19081,
            daemon_transport: String::new(),
            daemon_restricted_mode: false,
            daemon_login_enabled: false,
            daemon_login_username: String::new(),
            daemon_login_password: String::new(),
            wallet_rpc_enabled: false,
            wallet_ip: "127.0.0.1".to_string(),
            wallet_port: 19092,
            wallet_transport: String::new(),
            wallet_login_enabled: false,
            wallet_login_username: String::new(),
            wallet_login_password: String::new(),
            poll_frequency_seconds: 10,
        }
    }
}

impl Settings {
    pub fn load() -> Result<(Self, bool), Box<dyn std::error::Error>> {
        let path = "settings.json";
        if Path::new(path).exists() {
            let data = fs::read_to_string(path)?;
            let mut settings: Settings = serde_json::from_str(&data)?;
            settings.normalize();
            Ok((settings, true))
        } else {
            Ok((Self::default(), false))
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write("settings.json", data)?;
        Ok(())
    }

    pub fn daemon_connection(&self) -> RpcConnectionSettings {
        RpcConnectionSettings {
            endpoints: urls_for(&self.daemon_ip, self.daemon_port, &self.daemon_transport),
            username: if self.daemon_login_enabled {
                trimmed_or_none(&self.daemon_login_username)
            } else {
                None
            },
            password: if self.daemon_login_enabled {
                trimmed_or_none(&self.daemon_login_password)
            } else {
                None
            },
        }
    }

    pub fn wallet_connection(&self) -> RpcConnectionSettings {
        RpcConnectionSettings {
            endpoints: urls_for(&self.wallet_ip, self.wallet_port, &self.wallet_transport),
            username: if self.wallet_login_enabled {
                trimmed_or_none(&self.wallet_login_username)
            } else {
                None
            },
            password: if self.wallet_login_enabled {
                trimmed_or_none(&self.wallet_login_password)
            } else {
                None
            },
        }
    }

    pub fn daemon_url_display(&self) -> String {
        display_urls(&self.daemon_ip, self.daemon_port, &self.daemon_transport)
    }

    pub fn wallet_url_display(&self) -> String {
        display_urls(&self.wallet_ip, self.wallet_port, &self.wallet_transport)
    }

    fn normalize(&mut self) {
        self.daemon_transport = normalize_transport(&self.daemon_transport, "http");
        self.wallet_transport = normalize_transport(&self.wallet_transport, "https");
    }
}

fn trimmed_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_transport(value: &str, fallback: &str) -> String {
    let lowered = value.trim().to_ascii_lowercase();
    match lowered.as_str() {
        "http" | "https" => lowered,
        _ => fallback.to_string(),
    }
}

fn urls_for(ip: &str, port: u16, transport: &str) -> Vec<String> {
    match normalize_transport(transport, "http").as_str() {
        "http" => vec![format!("http://{ip}:{port}/json_rpc")],
        "https" => vec![format!("https://{ip}:{port}/json_rpc")],
        _ => vec![format!("http://{ip}:{port}/json_rpc")],
    }
}

fn display_urls(ip: &str, port: u16, transport: &str) -> String {
    match normalize_transport(transport, "http").as_str() {
        "http" => format!("http://{ip}:{port}/json_rpc"),
        "https" => format!("https://{ip}:{port}/json_rpc"),
        _ => format!("http://{ip}:{port}/json_rpc"),
    }
}
