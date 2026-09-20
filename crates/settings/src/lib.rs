#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub addr: String,
}

impl ServerSettings {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}
