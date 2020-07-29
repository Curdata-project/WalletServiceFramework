use jsonrpc_lite::{Error as JsonRpcError, JsonRpc};
use serde_json::Value;

pub const JSONRPC_ERROR_DEFAULT: i64 = 9999i64;

pub fn jsonrpc_error_to_value(err: JsonRpcError) -> Value {
    serde_json::to_value(JsonRpc::error((), err)).unwrap()
}

pub fn jsonrpc_id_error_to_value(id: i64, err: JsonRpcError) -> Value {
    serde_json::to_value(JsonRpc::error(id, err)).unwrap()
}
