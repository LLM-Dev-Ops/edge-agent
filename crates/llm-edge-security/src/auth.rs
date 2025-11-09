//! Authentication implementations

use crate::{SecurityError, SecurityResult};
use secrecy::{ExposeSecret, Secret};
use std::collections::HashMap;

/// API key authentication
pub struct ApiKeyAuth {
    keys: HashMap<String, Secret<String>>,
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyAuth {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    pub fn add_key(&mut self, client_id: String, api_key: String) {
        self.keys.insert(client_id, Secret::new(api_key));
    }

    pub fn validate(&self, api_key: &str) -> SecurityResult<String> {
        for (client_id, secret) in &self.keys {
            if secret.expose_secret() == api_key {
                return Ok(client_id.clone());
            }
        }
        Err(SecurityError::InvalidApiKey)
    }
}

/// JWT token authentication
pub struct JwtAuth {
    // TODO: Implement JWT validation
}

impl Default for JwtAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtAuth {
    pub fn new() -> Self {
        Self {}
    }

    pub fn validate(&self, _token: &str) -> SecurityResult<String> {
        // TODO: Implement JWT validation
        todo!("JWT validation not yet implemented")
    }
}
