use crate::WebSocketModule;
use actix::prelude::*;
use ewf_core::error::Error as EwfError;
use ewf_core::Call;
use jsonrpc_lite::{Error as JsonRpcError, JsonRpc};
use serde::Deserialize;
use serde_json::Value;

const JSONRPC_ERROR_DEFAULT: i64 = 9999i64;

fn jsonrpc_error_to_value(err: JsonRpcError) -> Value {
    serde_json::to_value(JsonRpc::error((), err)).unwrap()
}

fn jsonrpc_id_error_to_value(id: i64, err: JsonRpcError) -> Value {
    serde_json::to_value(JsonRpc::error(id, err)).unwrap()
}

#[derive(Deserialize, Debug)]
struct Request {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) params: Value,
    pub(crate) id: i64,
}

/// 传入一个Value格式的json-rpc单独请求
///   立刻返回响应执行Future或者错误结果
async fn route_once(redirecter: Addr<WebSocketModule>, req: Value) -> Value {
    let req: Request = match serde_json::from_value(req) {
        Ok(req) => req,
        Err(_) => return jsonrpc_error_to_value(JsonRpcError::invalid_request()),
    };

    let resp = match redirecter
        .send(Call {
            method: req.method,
            args: req.params,
        })
        .await
    {
        Ok(Ok(resp)) => serde_json::to_value(JsonRpc::success(req.id, &resp)).unwrap(),
        Ok(Err(EwfError::MethodNotFoundError)) => {
            jsonrpc_id_error_to_value(req.id, JsonRpcError::method_not_found())
        }
        Ok(Err(EwfError::CallParamValidFaild)) => {
            jsonrpc_id_error_to_value(req.id, JsonRpcError::invalid_params())
        }
        Ok(Err(EwfError::OtherError(other_err))) => jsonrpc_id_error_to_value(
            req.id,
            JsonRpcError {
                code: JSONRPC_ERROR_DEFAULT,
                message: format!("{}", other_err),
                data: None,
            },
        ),
        Ok(Err(EwfError::JsonRpcError { code, msg })) => jsonrpc_id_error_to_value(
            req.id,
            JsonRpcError {
                code: code,
                message: msg,
                data: None,
            },
        ),
        Ok(Err(err)) => jsonrpc_id_error_to_value(
            req.id,
            JsonRpcError {
                code: JSONRPC_ERROR_DEFAULT,
                message: format!("{:?}", err),
                data: None,
            },
        ),
        // 投递错误
        Err(_) => jsonrpc_id_error_to_value(req.id, JsonRpcError::internal_error()),
    };

    resp
}

/// 传入jsonrpc请求
///   返回结果
pub(crate) async fn route_jsonrpc(redirecter: Addr<WebSocketModule>, req_str: &str) -> String {
    let req: Value = match serde_json::from_str(req_str) {
        Ok(req) => req,
        Err(_) => return jsonrpc_error_to_value(JsonRpcError::parse_error()).to_string(),
    };
    let resp = match req {
        Value::Object(_) => route_once(redirecter, req).await.to_string(),
        Value::Array(array) => {
            let mut tasks = Vec::new();

            if array.len() == 0 {
                return jsonrpc_error_to_value(JsonRpcError::invalid_request()).to_string();
            }

            for each in array {
                tasks.push(async { route_once(redirecter.clone(), each).await });
            }
            let output = futures_util::future::join_all(tasks).await;

            Value::Array(output).to_string()
        }
        _ => return jsonrpc_error_to_value(JsonRpcError::parse_error()).to_string(),
    };

    resp.to_string()
}
