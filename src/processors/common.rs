use worker::*;

/// Fetches the actual Cloudflare datacenter (colo) where code is executing
/// by querying the Cloudflare trace endpoint.
///
/// Returns the 3-letter airport code (e.g., "SJC", "IAD", "LHR") or "unknown" on error.
pub async fn fetch_actual_colo() -> Result<String> {
    // Create a new fetch request to Cloudflare's trace endpoint
    let mut init = RequestInit::new();
    init.method = Method::Get;

    let request = Request::new_with_init("https://cloudflare.com/cdn-cgi/trace", &init)?;

    let mut trace_response = Fetch::Request(request).send().await?;
    let trace_text = trace_response.text().await?;

    // Parse the trace response to find the colo value
    // The format is key=value pairs separated by newlines
    for line in trace_text.lines() {
        if let Some(colo) = line.strip_prefix("colo=") {
            // Extract the 3-letter airport code
            let colo_code = colo.trim().to_string();
            if !colo_code.is_empty() {
                return Ok(colo_code);
            }
        }
    }

    Ok("unknown".to_string())
}

/// Helper function to get actual colo with error handling
pub async fn get_actual_colo() -> String {
    fetch_actual_colo().await.unwrap_or_else(|e| {
        console_log!("Failed to get actual colo: {}", e);
        "unknown".to_string()
    })
}
