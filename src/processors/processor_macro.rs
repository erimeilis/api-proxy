/// Macro to generate a regional processor with common functionality
///
/// Usage: `define_processor!(WNAMProcessor, "WNAM", "Western North America");`
#[macro_export]
macro_rules! define_processor {
    ($struct_name:ident, $region_code:expr, $region_name:expr) => {
        use worker::*;
        use crate::processors::common;
        use crate::handlers;
        use crate::logger;

        // Durable Object that processes requests in a specific region
        #[durable_object]
        pub struct $struct_name {
            #[allow(dead_code)]
            state: State,
            #[allow(dead_code)]
            env: Env,
        }

        impl DurableObject for $struct_name {
            fn new(state: State, env: Env) -> Self {
                Self { state, env }
            }

            async fn fetch(&self, mut req: Request) -> Result<Response> {
                // Read log level from header
                let log_level = logger::LogLevel::from_header(
                    &req.headers().get("X-Log-Level")?.unwrap_or_default()
                );

                // Get the actual datacenter where this DO is executing
                let actual_colo = common::get_actual_colo().await;
                log_info!(
                    "{} processing in datacenter: {} (Region: {})",
                    stringify!($struct_name),
                    actual_colo,
                    $region_name
                );

                // Check X-Request-Type header to determine SOAP vs HTTP
                let request_type = req.headers().get("X-Request-Type")?.unwrap_or_default();
                let is_soap = request_type.to_lowercase() == "soap";

                if is_soap {
                    // Handle SOAP request
                    log_info!("Processing SOAP request");

                    let soap_request_data = match req.json::<handlers::SoapRequestData>().await {
                        Ok(data) => {
                            log_debug!(log_level, "SOAP action: {}, namespace: {}, url: {}", data.action, data.namespace, data.url);
                            data
                        }
                        Err(e) => {
                            log_error!("Failed to parse SOAP request JSON: {}", e);
                            return Response::error(format!("Invalid SOAP JSON: {}", e), 400);
                        }
                    };

                    // Process the SOAP request
                    match handlers::process_soap_request(soap_request_data, log_level).await {
                        Ok(api_response) => {
                            log_info!("SOAP request completed successfully");
                            Response::from_json(&api_response)
                        }
                        Err(e) => {
                            log_error!("SOAP request processing error: {}", e);
                            Response::error(format!("SOAP error: {}", e), 500)
                        }
                    }
                } else {
                    // Handle regular HTTP request
                    log_info!("Processing HTTP request");

                    let request_data = match req.json::<handlers::RequestData>().await {
                        Ok(data) => {
                            log_debug!(log_level, "HTTP method: {:?}, url: {}", data.method, data.url);
                            data
                        }
                        Err(e) => {
                            log_error!("Failed to parse request JSON: {}", e);
                            return Response::error(format!("Invalid JSON: {}", e), 400);
                        }
                    };

                    // Process the proxy request
                    match handlers::process_request(request_data, log_level).await {
                        Ok(api_response) => {
                            log_info!("HTTP request completed successfully");
                            Response::from_json(&api_response)
                        }
                        Err(e) => {
                            log_error!("Proxy request processing error: {}", e);
                            Response::error(format!("Proxy error: {}", e), 500)
                        }
                    }
                }
            }
        }
    };
}
