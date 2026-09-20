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
