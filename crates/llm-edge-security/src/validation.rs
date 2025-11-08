//! Request validation

use crate::{SecurityError, SecurityResult};

/// Validates request size limits
pub fn validate_request_size(size: usize, max_size: usize) -> SecurityResult<()> {
    if size > max_size {
        Err(SecurityError::Validation(format!(
            "Request size {} exceeds maximum {}",
            size, max_size
        )))
    } else {
        Ok(())
    }
}

/// Validates temperature parameter
pub fn validate_temperature(temp: f32) -> SecurityResult<()> {
    if !(0.0..=2.0).contains(&temp) {
        Err(SecurityError::Validation(format!(
            "Temperature {} must be between 0.0 and 2.0",
            temp
        )))
    } else {
        Ok(())
    }
}

/// Validates max_tokens parameter
pub fn validate_max_tokens(tokens: usize, max: usize) -> SecurityResult<()> {
    if tokens == 0 || tokens > max {
        Err(SecurityError::Validation(format!(
            "max_tokens {} must be between 1 and {}",
            tokens, max
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_validation() {
        assert!(validate_temperature(1.0).is_ok());
        assert!(validate_temperature(0.0).is_ok());
        assert!(validate_temperature(2.0).is_ok());
        assert!(validate_temperature(-0.1).is_err());
        assert!(validate_temperature(2.1).is_err());
    }

    #[test]
    fn test_max_tokens_validation() {
        assert!(validate_max_tokens(100, 1000).is_ok());
        assert!(validate_max_tokens(0, 1000).is_err());
        assert!(validate_max_tokens(1001, 1000).is_err());
    }
}
