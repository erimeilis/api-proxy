use worker::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod auth;
mod handlers;
#[macro_use]
mod logger;

#[macro_use]
mod processors;

// Re-export all processors so they're accessible to the worker runtime
pub use processors::wnam_processor::WNAMProcessor;
pub use processors::enam_processor::ENAMProcessor;
pub use processors::weur_processor::WEURProcessor;
pub use processors::eeur_processor::EEURProcessor;
pub use processors::apac_processor::APACProcessor;
pub use processors::oc_processor::OCProcessor;
pub use processors::af_processor::AFProcessor;
pub use processors::me_processor::MEProcessor;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<HttpResponse> {
    // Convert HttpRequest to worker::Request using try_from
    let mut worker_req = Request::try_from(req)?;

    // Validate authentication token before processing
    if let Err(_) = auth::validate_token(&worker_req, &env) {
        return auth::AuthError::forbidden()?.try_into();
    }

    // Read X-Log-Level header to determine logging level
    let log_level = logger::LogLevel::from_header(
        &worker_req
            .headers()
            .get("X-Log-Level")?
            .unwrap_or_default()
    );

    // Get the datacenter where main worker is executing
    let colo = worker_req.cf().map(|cf| cf.colo()).unwrap_or("unknown".to_string());
    log_info!("Request received at datacenter: {}", colo);

    // Get the original URL path
    let url = worker_req.url()?;
    let path = url.path().to_string();
    log_debug!(log_level, "Request path: {}", path);

    // Read X-CF-Region header to determine target region
    let region_header = worker_req
        .headers()
        .get("X-CF-Region")?
        .unwrap_or_else(|| "wnam".to_string()); // Default to Western North America

    log_info!("Selected region: {}", region_header);

    // Read X-Request-Type header (soap or http)
    let request_type = worker_req
        .headers()
        .get("X-Request-Type")?
        .unwrap_or_default();

    // Parse incoming request body
    let body_text = worker_req.text().await?;

    // Map header value to ProcessorRegion
    let region = match region_header.to_lowercase().as_str() {
        "wnam" => ProcessorRegion::WesternNorthAmerica,
        "enam" => ProcessorRegion::EasternNorthAmerica,
        "weur" => ProcessorRegion::WesternEurope,
        "eeur" => ProcessorRegion::EasternEurope,
        "apac" => ProcessorRegion::AsiaPacific,
        "oc" => ProcessorRegion::Oceania,
        "af" => ProcessorRegion::Africa,
        "me" => ProcessorRegion::MiddleEast,
        _ => {
            log_info!("Unknown region '{}', defaulting to Western North America", region_header);
            ProcessorRegion::WesternNorthAmerica
        }
    };

    // Route to the appropriate regional processor
    route_to_processor(&env, &path, body_text, region, &request_type, log_level).await?.try_into()
}

/// Route request to appropriate regional processor based on location
///
/// Uses hash-based distribution across 10 Durable Objects per region for 10x concurrency.
///
/// EU Jurisdiction Enforcement:
/// For GDPR compliance, Western and Eastern Europe processors use location hints
/// "weur" and "eeur" which Cloudflare automatically maps to EU datacenters,
/// enforcing data residency within EU jurisdiction.
async fn route_to_processor(
    env: &Env,
    path: &str,
    body: String,
    region: ProcessorRegion,
    request_type: &str,
    log_level: logger::LogLevel,
) -> Result<Response> {
    // Calculate hash-based DO index (0-9) for load distribution
    let mut hasher = DefaultHasher::new();
    body.hash(&mut hasher);
    let hash_value = hasher.finish();
    let do_index = (hash_value % 10) as u32;

    let (namespace_name, region_code, location_hint, is_eu) = match region {
        ProcessorRegion::WesternNorthAmerica => ("WNAM_PROCESSOR", "wnam", "wnam", false),
        ProcessorRegion::EasternNorthAmerica => ("ENAM_PROCESSOR", "enam", "enam", false),
        ProcessorRegion::WesternEurope => ("WEUR_PROCESSOR", "weur", "weur", true),
        ProcessorRegion::EasternEurope => ("EEUR_PROCESSOR", "eeur", "eeur", true),
        ProcessorRegion::AsiaPacific => ("APAC_PROCESSOR", "apac", "apac", false),
        ProcessorRegion::Oceania => ("OC_PROCESSOR", "oc", "oc", false),
        ProcessorRegion::Africa => ("AF_PROCESSOR", "af", "af", false),
        ProcessorRegion::MiddleEast => ("ME_PROCESSOR", "me", "me", false),
    };

    let do_name = format!("{}-processor-{}", region_code, do_index);

    log_debug!(
        log_level,
        "Routing to {} ({}) with location hint: {} (EU jurisdiction: {})",
        namespace_name,
        do_name,
        location_hint,
        is_eu
    );

    // Get the Durable Object namespace
    let namespace = env.durable_object(namespace_name)?;

    // Get DO stub with location hint
    // For EU regions (weur/eeur), the location hint enforces EU jurisdiction automatically
    // This ensures GDPR compliance by keeping data within EU datacenters
    let stub = namespace.get_by_name_with_location_hint(&do_name, location_hint)?;

    // Create internal request URL preserving the path
    let internal_url = format!("http://internal{}", path);

    // Create headers and forward X-Request-Type and X-Log-Level to Durable Object
    let headers = worker::Headers::new();
    headers.set("Content-Type", "application/json")?;
    if !request_type.is_empty() {
        headers.set("X-Request-Type", request_type)?;
    }
    headers.set("X-Log-Level", if log_level == logger::LogLevel::Debug { "debug" } else { "info" })?;

    // Forward request to Durable Object
    let mut init = RequestInit::new();
    init.method = Method::Post;
    init.headers = headers;
    init.body = Some(body.into());

    let do_request = Request::new_with_init(&internal_url, &init)?;

    stub.fetch_with_request(do_request).await
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum ProcessorRegion {
    WesternNorthAmerica,
    EasternNorthAmerica,
    WesternEurope,
    EasternEurope,
    AsiaPacific,
    Oceania,
    Africa,
    MiddleEast
}
