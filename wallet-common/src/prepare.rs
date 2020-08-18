use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModStatus {
    UnInital,
    InitalSuccess,
    InitalFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModStatusPullParam {
    pub mod_name: String,
    pub is_prepare: ModStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInitialParam {
    
}
