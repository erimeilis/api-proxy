use anyhow::Context as AnyhowContext;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use worker::*;

#[derive(Debug, Deserialize)]
pub struct SoapRequestData {
    /// URL to send the SOAP request to
    pub url: String,

    /// SOAP action/method name (e.g., "getDIDCountry")
    pub action: String,

    /// Namespace for the SOAP action (e.g., "urn:getDIDCountry")
    pub namespace: String,

    /// Parameters as ordered array of [key, value] pairs (preserves order)
    #[serde(default)]
    pub params: Vec<(String, Value)>,

    /// Request headers as key-value pairs
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    #[allow(dead_code)]
    pub timeout: u64,
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

/// Process a SOAP request by building SOAP envelope and forwarding to target URL
pub async fn process_soap_request(data: SoapRequestData) -> anyhow::Result<ApiResponse> {
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

    // Add SOAP-specific headers that match nusoap exactly
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("text/xml; charset=ISO-8859-1"),
    );
    headers.insert(
        HeaderName::from_static("soapaction"),
        HeaderValue::from_static("\"\""), // Empty SOAPAction like nusoap
    );
    headers.insert(
        HeaderName::from_static("user-agent"),
        HeaderValue::from_static("NuSOAP/0.9.17 (1.123)"), // Match nusoap exactly
    );

    // Build SOAP body content with namespace prefix (like nusoap does)
    // Use ns1766 as the namespace prefix to match nusoap format exactly
    let mut soap_body_content = format!(
        "<ns1766:{} xmlns:ns1766=\"{}\">",
        data.action, data.namespace
    );

    // Add parameters with type hints
    // Match nusoap behavior: numeric keys become __numeric_N
    // Vec preserves exact order from Laravel
    for (key, value) in &data.params {
        // Check if key is numeric and convert to __numeric_N format like nusoap
        let xml_key = if key.chars().all(|c| c.is_numeric()) {
            format!("__numeric_{}", key)
        } else {
            key.clone()
        };

        let (type_hint, value_str) = match value {
            Value::Bool(b) => ("xsd:boolean", b.to_string()),
            Value::Number(n) => ("xsd:int", n.to_string()),
            Value::String(s) => ("xsd:string", html_escape(s)),
            Value::Null => ("xsd:string", String::new()),
            _ => ("xsd:string", value.to_string()),
        };

        soap_body_content.push_str(&format!(
            "<{} xsi:type=\"{}\">{}</{}>",
            xml_key, type_hint, value_str, xml_key
        ));
    }

    soap_body_content.push_str(&format!("</ns1766:{}>", data.action));

    // Construct complete SOAP envelope - DidX needs the EXACT format that nusoap sends
    // CRITICAL: Must be single line with NO newlines (except XML declaration)
    let soap_envelope = format!(
        "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><SOAP-ENV:Envelope SOAP-ENV:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\" xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\"><SOAP-ENV:Body>{}</SOAP-ENV:Body></SOAP-ENV:Envelope>",
        soap_body_content
    );

    console_log!(
        "Sending SOAP request to {} with action {}, namespace {}, params {:?}",
        data.url,
        data.action,
        data.namespace,
        data.params
    );

    // Build and send the request
    let request = client.post(&data.url).headers(headers).body(soap_envelope);

    // Send the request
    let response = request.send().await.context("Failed to send SOAP request")?;

    // Process the response
    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown Status");

    console_log!(
        "Received SOAP response with status: {} ({})",
        status,
        status_text
    );

    // Check if it's a success status (200-299)
    if (200..300).contains(&status) {
        // Convert response headers to HashMap
        let mut header_map = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                header_map.insert(key.to_string(), v.to_string());
            }
        }

        // Get the response text
        let text = response
            .text()
            .await
            .context("Failed to read SOAP response body")?;

        // Return the SOAP XML response as a string
        let body = serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|_| Value::String(text.clone()));

        console_log!("SOAP Response headers: {:?}", &header_map);
        console_log!("SOAP Response body: {}", text);

        Ok(ApiResponse::Success(ResponseData {
            status,
            headers: header_map,
            body,
        }))
    } else {
        console_log!("SOAP Error response: status {} - {}", status, status_text);

        Ok(ApiResponse::Error(ErrorResponseData {
            status,
            message: status_text.to_string(),
        }))
    }
}

/// HTML escape helper for SOAP parameter values
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
