use std::env;

#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub addr: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseSettings {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct StorageSettings {
    pub static_root: String,
}

impl ServerSettings {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    pub fn from_env(var_name: &str, default_addr: &str) -> Self {
        Self {
            addr: env::var(var_name).unwrap_or_else(|_| default_addr.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageSettings {
    pub static_root: String,
}

impl StorageSettings {
    pub fn new(static_root: impl Into<String>) -> Self {
        Self {
            static_root: static_root.into(),
        }
    }

    pub fn from_env(var_name: &str, default_root: &str) -> Self {
        Self {
            static_root: env::var(var_name).unwrap_or_else(|_| default_root.to_string()),
        }
    }
}

impl DatabaseSettings {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl StorageSettings {
    pub fn new(static_root: impl Into<String>) -> Self {
        Self {
            static_root: static_root.into(),
        }
    }
}

impl DatabaseSettings {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl StorageSettings {
    pub fn new(static_root: impl Into<String>) -> Self {
        Self {
            static_root: static_root.into(),
        }
    }
}
