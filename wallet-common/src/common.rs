use serde::{Deserialize, Serialize};


///
/// 暂只支持分页
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParam{
    pub page_items: u32,
    pub page_num: u32,
    pub order_by: String,
}
