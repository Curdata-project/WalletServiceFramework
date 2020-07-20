
pub mod error;


use std::path::Path;
use wallet_service_framework::Error as FrameworkError;
use wallet_service_framework::{Bus, Event, Module};
use serde_json::{Value, json};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use std::time::Duration;
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Method {
    Get,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest{
    url: String,
    method: Method,
    data: String,
}

pub struct ReqwestCli {
}

impl ReqwestCli {
    pub fn new(path: String) -> Result<Self, FrameworkError> {
        Ok(Self {
        })
    }

    pub async fn request(&self, req: Value) -> Value {
        let req: HttpRequest = serde_json::from_value(req).unwrap();
        let reqwest_method = match req.method {
            Method::Get => http::Method::GET,
            Method::Post => http::Method::POST,
        };

        // 考虑reuse
        return match reqwest::Client::new()
            .request(http::Method::POST, &req.url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(req.data)
            .timeout(Duration::new(5, 0))
            .send()
            .await
        {
            Err(err) => {
                return json!({
                    "code": if err.is_builder() { 417u16 }
                    else if err.is_redirect() { 300u16 }
                    else if err.is_timeout() { 404u16 }
                    else if err.is_status() { 591u16 }
                    else { 592u16 }
                })
            }
            Ok(resp) => {
                match resp.text().await {
                    Ok(resp) => {
                        json!({"code": 0u16, "data": resp})
                    }
                    Err(err) => {
                        json!({"code": 593u16})
                    }
                }
            }
        };
    }
}

impl Module for ReqwestCli {
    fn event_call(&self, bus: &Bus, event: &Event) -> Result<(), FrameworkError> {
        let event: &str = &event.event;
        match event {
            // no care this event, ignore
            _ => return Ok(()),
        }

        Ok(())
    }

    fn call(&self, method: &str, _intput: Value) -> Result<Value, FrameworkError> {
        match method {
            "request" => {
                async move {
                    let resp = self.request(_intput);
                };
                Ok(Value::Null)
            }
            _ => Err(FrameworkError::MethodNotFoundError),
        }
    }

    fn name(&self) -> String {
        "keypair".to_string()
    }

    fn version(&self) -> String {
        "0.1.0".to_string()
    }
}
