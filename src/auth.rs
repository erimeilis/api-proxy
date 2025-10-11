use worker::*;

/// Authentication error responses
pub struct AuthError;

impl AuthError {
    /// Returns a 403 Forbidden response for authentication failures
    pub fn forbidden() -> Result<Response> {
        Response::error("Forbidden: Invalid or missing authentication token", 403)
    }
}

/// Validates the authentication token from the Authorization header
///
/// Expected header format: `Authorization: Bearer <token>`
///
/// Returns Ok(()) if the token is valid, Err(AuthError) otherwise
pub fn validate_token(req: &Request, env: &Env) -> Result<()> {
    // Get the expected token from environment variable
    let expected_token = env.secret("AUTH_TOKEN")?.to_string();

    // Get the Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")?
        .ok_or_else(|| {
            console_log!("Authentication failed: Missing Authorization header");
            worker::Error::RustError("Missing Authorization header".to_string())
        })?;

    // Check if it starts with "Bearer "
    if !auth_header.starts_with("Bearer ") {
        console_log!("Authentication failed: Invalid Authorization header format");
        return Err(worker::Error::RustError("Invalid Authorization header format".to_string()));
    }

    // Extract the token
    let token = auth_header.strip_prefix("Bearer ").unwrap_or("");

    // Validate the token
    if token != expected_token {
        console_log!("Authentication failed: Invalid token");
        return Err(worker::Error::RustError("Invalid token".to_string()));
    }

    console_log!("Authentication successful");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_token_parsing() {
        let auth_header = "Bearer test-token-123";
        assert!(auth_header.starts_with("Bearer "));
        let token = auth_header.strip_prefix("Bearer ").unwrap();
        assert_eq!(token, "test-token-123");
    }
}
