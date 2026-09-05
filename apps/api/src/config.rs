use std::{env, net::SocketAddr, str::FromStr};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_address: SocketAddr,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, std::net::AddrParseError> {
        let raw_address =
            env::var("API_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());

        SocketAddr::from_str(&raw_address).map(|bind_address| Self { bind_address })
    }
}
