use reqwest::header::{HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};
use std::time::Duration;

pub async fn reqwest_json(url: &str, req: Value, timeout: u64) -> Result<Value, String> {
    match reqwest::Client::new()
        .request(http::Method::POST, url)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(req.to_string())
        .timeout(Duration::new(timeout, 0))
        .send()
        .await
    {
        Err(err) => Err(format!("HttpReqError {}", err.to_string())),
        Ok(resp) => {
            let resp: Value = serde_json::from_str(&resp.text().await.expect("resp not string"))
                .map_err(|_| "HttpResponseNotJson")?;
            Ok(resp)
        }
    }
}
