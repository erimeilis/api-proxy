/// Macro to generate a regional processor with common functionality
///
/// Usage: `define_processor!(WNAMProcessor, "WNAM", "Western North America");`
#[macro_export]
macro_rules! define_processor {
    ($struct_name:ident, $region_code:expr, $region_name:expr) => {
        use worker::*;
        use crate::processors::common;
        use crate::handlers;

        // Durable Object that processes requests in a specific region
        #[durable_object]
        pub struct $struct_name {
            state: State,
            env: Env,
        }

        impl DurableObject for $struct_name {
            fn new(state: State, env: Env) -> Self {
                Self { state, env }
            }

            async fn fetch(&self, mut req: Request) -> Result<Response> {
                // Get the actual datacenter where this DO is executing
                let actual_colo = common::get_actual_colo().await;
                console_log!(
                    "{} executing in datacenter: {} (Region: {})",
                    stringify!($struct_name),
                    actual_colo,
                    $region_name
                );

                // Check X-Request-Type header to determine SOAP vs HTTP
                let request_type = req.headers().get("X-Request-Type")?.unwrap_or_default();
                let is_soap = request_type.to_lowercase() == "soap";

                if is_soap {
                    // Handle SOAP request
                    console_log!("Processing SOAP request (X-Request-Type: soap)");

                    let soap_request_data = match req.json::<handlers::SoapRequestData>().await {
                        Ok(data) => {
                            console_log!("Received SOAP request data: action={}, namespace={}", data.action, data.namespace);
                            data
                        }
                        Err(e) => {
                            console_log!("Failed to parse SOAP request JSON: {}", e);
                            return Response::error(format!("Invalid SOAP JSON: {}", e), 400);
                        }
                    };

                    // Process the SOAP request
                    match handlers::process_soap_request(soap_request_data).await {
                        Ok(api_response) => {
                            console_log!("Successfully processed SOAP request");
                            Response::from_json(&api_response)
                        }
                        Err(e) => {
                            console_log!("SOAP request processing error: {}", e);
                            Response::error(format!("SOAP error: {}", e), 500)
                        }
                    }
                } else {
                    // Handle regular HTTP request
                    console_log!("Processing regular HTTP request");

                    let request_data = match req.json::<handlers::RequestData>().await {
                        Ok(data) => {
                            console_log!("Received request data for URL: {}", data.url);
                            data
                        }
                        Err(e) => {
                            console_log!("Failed to parse request JSON: {}", e);
                            return Response::error(format!("Invalid JSON: {}", e), 400);
                        }
                    };

                    // Process the proxy request
                    match handlers::process_request(request_data).await {
                        Ok(api_response) => {
                            console_log!("Successfully processed proxy request");
                            Response::from_json(&api_response)
                        }
                        Err(e) => {
                            console_log!("Proxy request processing error: {}", e);
                            Response::error(format!("Proxy error: {}", e), 500)
                        }
                    }
                }
            }
        }
    };
}
