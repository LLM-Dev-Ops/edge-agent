# llm-edge-security

[![Crates.io](https://img.shields.io/crates/v/llm-edge-security.svg)](https://crates.io/crates/llm-edge-security)
[![Documentation](https://docs.rs/llm-edge-security/badge.svg)](https://docs.rs/llm-edge-security)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE)

Security layer for LLM Edge Agent, providing authentication, input validation, and PII (Personally Identifiable Information) detection and redaction capabilities.

## Features

- **API Key Authentication**: Secure client authentication using API keys with secret management
- **JWT Token Validation**: JSON Web Token authentication support (planned)
- **PII Detection & Redaction**: Automatic detection and redaction of sensitive information
  - Social Security Numbers (SSN)
  - Email addresses
  - Credit card numbers
- **Input Validation**: Comprehensive request parameter validation
  - Request size limits
  - Temperature parameter validation (0.0-2.0)
  - Max tokens validation
- **Type-Safe Error Handling**: Strongly typed security errors using `thiserror`
- **Zero-Copy Operations**: Efficient security checks with minimal overhead

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
llm-edge-security = "0.1.0"
```

Or use `cargo add`:

```bash
cargo add llm-edge-security
```

## Usage

### API Key Authentication

```rust
use llm_edge_security::{ApiKeyAuth, SecurityResult};

fn main() -> SecurityResult<()> {
    // Initialize API key authenticator
    let mut auth = ApiKeyAuth::new();

    // Add API keys for clients
    auth.add_key("client-1".to_string(), "sk-1234567890abcdef".to_string());
    auth.add_key("client-2".to_string(), "sk-fedcba0987654321".to_string());

    // Validate incoming API key
    let api_key = "sk-1234567890abcdef";
    match auth.validate(api_key) {
        Ok(client_id) => {
            println!("Authenticated client: {}", client_id);
            Ok(())
        }
        Err(e) => {
            eprintln!("Authentication failed: {}", e);
            Err(e)
        }
    }
}
```

### PII Detection and Redaction

```rust
use llm_edge_security::PIIRedactor;

fn main() {
    let redactor = PIIRedactor::new();

    // Example text with PII
    let text = "Contact John at john.doe@example.com or call 123-45-6789. \
                Credit card: 4532-1234-5678-9010";

    // Check if text contains PII
    if redactor.contains_pii(text) {
        println!("Warning: PII detected in input");

        // Redact sensitive information
        let safe_text = redactor.redact(text);
        println!("Redacted: {}", safe_text);
        // Output: "Contact John at [EMAIL_REDACTED] or call [SSN_REDACTED].
        //          Credit card: [CC_REDACTED]"
    }
}
```

### Input Validation

```rust
use llm_edge_security::validation::{
    validate_request_size,
    validate_temperature,
    validate_max_tokens,
};
use llm_edge_security::SecurityResult;

fn validate_llm_request(
    request_size: usize,
    temperature: f32,
    max_tokens: usize,
) -> SecurityResult<()> {
    // Validate request size (max 1MB)
    validate_request_size(request_size, 1_048_576)?;

    // Validate temperature parameter
    validate_temperature(temperature)?;

    // Validate max tokens (max 4096)
    validate_max_tokens(max_tokens, 4096)?;

    Ok(())
}

fn main() -> SecurityResult<()> {
    // Valid request
    validate_llm_request(1024, 0.7, 2048)?;

    // Invalid temperature (will return error)
    match validate_llm_request(1024, 2.5, 2048) {
        Ok(_) => println!("Valid"),
        Err(e) => eprintln!("Validation error: {}", e),
    }

    Ok(())
}
```

### Error Handling

```rust
use llm_edge_security::{SecurityError, SecurityResult};

fn handle_security_error(result: SecurityResult<String>) {
    match result {
        Ok(client_id) => println!("Success: {}", client_id),
        Err(SecurityError::InvalidApiKey) => {
            eprintln!("Invalid API key provided");
        }
        Err(SecurityError::Validation(msg)) => {
            eprintln!("Validation failed: {}", msg);
        }
        Err(SecurityError::RateLimitExceeded) => {
            eprintln!("Rate limit exceeded, please try again later");
        }
        Err(e) => eprintln!("Security error: {}", e),
    }
}
```

## PII Patterns

The PII redactor currently detects and redacts the following patterns:

| Pattern | Regex | Replacement |
|---------|-------|-------------|
| **SSN** | `\b\d{3}-\d{2}-\d{4}\b` | `[SSN_REDACTED]` |
| **Email** | `\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z\|a-z]{2,}\b` | `[EMAIL_REDACTED]` |
| **Credit Card** | `\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b` | `[CC_REDACTED]` |

## Security Best Practices

When using this crate in production:

1. **API Key Management**
   - Store API keys in environment variables or secret management systems
   - Never hardcode API keys in source code
   - Rotate API keys regularly
   - Use the `secrecy` crate's `Secret` type to prevent accidental logging

2. **PII Handling**
   - Always run PII detection before logging user inputs
   - Redact PII before storing in databases or sending to third-party services
   - Consider regulatory requirements (GDPR, CCPA, HIPAA)
   - Implement audit trails for PII access

3. **Input Validation**
   - Validate all user inputs before processing
   - Set appropriate limits for request sizes and token counts
   - Use type-safe validation functions to prevent injection attacks

4. **Rate Limiting**
   - Implement rate limiting per API key
   - Monitor for suspicious authentication patterns
   - Log failed authentication attempts

5. **Transport Security**
   - Always use HTTPS/TLS for API communication
   - Implement certificate pinning where appropriate
   - Use secure headers (HSTS, CSP, etc.)

## Architecture

```
llm-edge-security/
├── auth.rs         # Authentication implementations (API key, JWT)
├── pii.rs          # PII detection and redaction
├── validation.rs   # Input validation functions
├── error.rs        # Security error types
└── lib.rs          # Public API
```

## Dependencies

Core dependencies:
- `secrecy` - Secret management to prevent accidental exposure
- `jsonwebtoken` - JWT token validation
- `argon2` - Password hashing
- `validator` - Data validation
- `regex` - Pattern matching for PII detection
- `thiserror` - Error handling

## Roadmap

- [ ] JWT token validation implementation
- [ ] OAuth2/OIDC authentication support
- [ ] Additional PII patterns (phone numbers, IP addresses, etc.)
- [ ] Rate limiting middleware
- [ ] Security event logging
- [ ] Encryption utilities
- [ ] HMAC signature verification

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/LICENSE) for details.

## Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/globalbusinessadvisors/llm-edge-agent/blob/main/CONTRIBUTING.md) for details.

For security vulnerabilities, please email security@globalbusinessadvisors.com instead of filing a public issue.

## Repository

https://github.com/globalbusinessadvisors/llm-edge-agent
