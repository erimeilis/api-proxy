use anyhow::Context as AnyhowContext;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method as ReqwestMethod,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use worker::*;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl<'de> Deserialize<'de> for HttpMethod {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "get" => Ok(HttpMethod::Get),
            "post" => Ok(HttpMethod::Post),
            "put" => Ok(HttpMethod::Put),
            "delete" => Ok(HttpMethod::Delete),
            "patch" => Ok(HttpMethod::Patch),
            "head" => Ok(HttpMethod::Head),
            "options" => Ok(HttpMethod::Options),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["get", "post", "put", "delete", "patch", "head", "options"],
            )),
        }
    }
}

impl From<HttpMethod> for ReqwestMethod {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => ReqwestMethod::GET,
            HttpMethod::Post => ReqwestMethod::POST,
            HttpMethod::Put => ReqwestMethod::PUT,
            HttpMethod::Delete => ReqwestMethod::DELETE,
            HttpMethod::Patch => ReqwestMethod::PATCH,
            HttpMethod::Head => ReqwestMethod::HEAD,
            HttpMethod::Options => ReqwestMethod::OPTIONS,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RequestData {
    /// URL to request
    pub url: String,

    /// HTTP method
    #[serde(default = "default_method")]
    pub method: HttpMethod,

    /// Request parameters as key-value pairs
    #[serde(default)]
    pub params: HashMap<String, String>,

    /// Request headers as key-value pairs
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request timeout in seconds
    /// Note: Timeout is not used in WebAssembly but kept for API compatibility
    #[serde(default = "default_timeout")]
    #[allow(dead_code)]
    pub timeout: u64,
}

fn default_method() -> HttpMethod {
    HttpMethod::Post
}

fn default_timeout() -> u64 {
    30
}

#[derive(Serialize)]
pub struct ResponseData {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

#[derive(Serialize)]
pub struct ErrorResponseData {
    pub status: u16,
    pub message: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ApiResponse {
    Success(ResponseData),
    Error(ErrorResponseData),
}

/// Process an HTTP request by forwarding it to the target URL
pub async fn process_request(data: RequestData) -> anyhow::Result<ApiResponse> {
    // Create a client (timeout not supported in WebAssembly)
    let client = Client::builder()
        .build()
        .context("Failed to create HTTP client")?;

    // Parse headers
    let mut headers = HeaderMap::new();
    for (key, value) in &data.headers {
        let header_name =
            HeaderName::from_str(key).context(format!("Invalid header name: {}", key))?;
        let header_value =
            HeaderValue::from_str(value).context(format!("Invalid header value: {}", value))?;
        headers.insert(header_name, header_value);
    }

    // Add default headers if not already set
    if !headers.contains_key("user-agent") {
        headers.insert(
            HeaderName::from_static("user-agent"),
            HeaderValue::from_static("ApiProxy/1.0"),
        );
    }

    // Build and send the request
    let method: ReqwestMethod = data.method.into();
    let mut request = client.request(method, &data.url).headers(headers);

    // Add parameters based on method
    if matches!(
        data.method,
        HttpMethod::Get | HttpMethod::Head | HttpMethod::Delete
    ) {
        request = request.query(&data.params);
        console_log!(
            "Sending {:?} request to {} with query params: {:?}",
            data.method,
            data.url,
            &data.params
        );
    } else {
        request = request.json(&data.params);
        console_log!(
            "Sending {:?} request to {} with JSON body: {:?}",
            data.method,
            data.url,
            &data.params
        );
    }

    console_log!("Request headers: {:?}", &data.headers);

    // Send the request
    let response = request.send().await.context("Failed to send request")?;

    // Process the response
    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown Status");

    // Log the response status
    console_log!("Received response with status: {} ({})", status, status_text);

    // Check if it's a success status (200-299)
    if (200..300).contains(&status) {
        // For success responses, return the full response data

        // Convert response headers to HashMap
        let mut header_map = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                header_map.insert(key.to_string(), v.to_string());
            }
        }

        // Try to parse as JSON first
        let text = response
            .text()
            .await
            .context("Failed to read response body")?;
        let body = serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|_| Value::String(text.clone()));

        // Log the full response
        console_log!("Response headers: {:?}", &header_map);
        console_log!("Response body: {}", text);

        Ok(ApiResponse::Success(ResponseData {
            status,
            headers: header_map,
            body,
        }))
    } else {
        // For error responses, return only the status code and message
        console_log!("Error response: returning only status code and message");

        Ok(ApiResponse::Error(ErrorResponseData {
            status,
            message: status_text.to_string(),
        }))
    }
}
