use serde::{Deserialize, Serialize};

/// 查询条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParam {
    pub page_items: u32,
    pub page_num: u32,
    pub order_by: String,
    pub is_asc_order: bool,
}
