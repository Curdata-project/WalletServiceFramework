use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareParam {
    pub is_prepare: bool,
}
