use std::collections::HashMap;

pub struct RequestValidator {
    max_content_length: u64,
    max_header_size: usize,
}

impl RequestValidator {
    pub fn new(max_content_length: u64, max_header_size: usize) -> Self {
        Self {
            max_content_length,
            max_header_size,
        }
    }

    pub fn validate_content_length(&self, headers: &HashMap<String, String>) -> Result<(), String> {
        if let Some(value) = headers.get("content-length") {
            let length: u64 = value
                .parse()
                .map_err(|_| format!("Invalid Content-Length value: {}", value))?;
            if length > self.max_content_length {
                return Err(format!(
                    "Payload Too Large: Content-Length {} exceeds maximum allowed {}",
                    length, self.max_content_length
                ));
            }
        }
        Ok(())
    }

    pub fn validate_content_type(
        &self,
        method: &str,
        headers: &HashMap<String, String>,
    ) -> Result<(), String> {
        let requires_content_type =
            matches!(method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH");

        if requires_content_type && !headers.contains_key("content-type") {
            return Err(format!(
                "Bad Request: Content-Type header is required for {} requests",
                method.to_uppercase()
            ));
        }

        Ok(())
    }

    pub fn validate_header_size(&self, headers: &HashMap<String, String>) -> Result<(), String> {
        let total_size: usize = headers.iter().map(|(k, v)| k.len() + v.len()).sum();

        if total_size > self.max_header_size {
            return Err(format!(
                "Request Header Fields Too Large: total header size {} exceeds maximum allowed {}",
                total_size, self.max_header_size
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_validator() -> RequestValidator {
        RequestValidator::new(10_000_000, 50_000)
    }

    #[test]
    fn test_content_length_under_limit_passes() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-length".to_string(), "1024".to_string());
        assert!(validator.validate_content_length(&headers).is_ok());
    }

    #[test]
    fn test_content_length_over_limit_fails() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-length".to_string(), "20000000".to_string());
        let result = validator.validate_content_length(&headers);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Payload Too Large"));
    }

    #[test]
    fn test_content_length_missing_passes() {
        let validator = make_validator();
        let headers = HashMap::new();
        assert!(validator.validate_content_length(&headers).is_ok());
    }

    #[test]
    fn test_content_length_invalid_value_fails() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-length".to_string(), "not-a-number".to_string());
        let result = validator.validate_content_length(&headers);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid Content-Length"));
    }

    #[test]
    fn test_content_type_required_for_post_fails_when_missing() {
        let validator = make_validator();
        let headers = HashMap::new();
        let result = validator.validate_content_type("POST", &headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Content-Type header is required"));
    }

    #[test]
    fn test_content_type_required_for_put_fails_when_missing() {
        let validator = make_validator();
        let headers = HashMap::new();
        let result = validator.validate_content_type("PUT", &headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Content-Type header is required"));
    }

    #[test]
    fn test_content_type_required_for_patch_fails_when_missing() {
        let validator = make_validator();
        let headers = HashMap::new();
        let result = validator.validate_content_type("PATCH", &headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Content-Type header is required"));
    }

    #[test]
    fn test_content_type_not_required_for_get_passes() {
        let validator = make_validator();
        let headers = HashMap::new();
        assert!(validator.validate_content_type("GET", &headers).is_ok());
    }

    #[test]
    fn test_content_type_not_required_for_delete_passes() {
        let validator = make_validator();
        let headers = HashMap::new();
        assert!(validator.validate_content_type("DELETE", &headers).is_ok());
    }

    #[test]
    fn test_content_type_present_for_post_passes() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        assert!(validator.validate_content_type("POST", &headers).is_ok());
    }

    #[test]
    fn test_header_size_under_limit_passes() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("authorization".to_string(), "Bearer token123".to_string());
        assert!(validator.validate_header_size(&headers).is_ok());
    }

    #[test]
    fn test_header_size_over_limit_fails() {
        let validator = RequestValidator::new(10_000_000, 100);
        let mut headers = HashMap::new();
        headers.insert("x-large-header".to_string(), "A".repeat(200));
        let result = validator.validate_header_size(&headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Request Header Fields Too Large"));
    }

    #[test]
    fn test_content_length_exactly_at_limit_passes() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-length".to_string(), "10000000".to_string());
        assert!(validator.validate_content_length(&headers).is_ok());
    }

    #[test]
    fn test_content_length_one_over_limit_fails() {
        let validator = make_validator();
        let mut headers = HashMap::new();
        headers.insert("content-length".to_string(), "10000001".to_string());
        let result = validator.validate_content_length(&headers);
        assert!(result.is_err());
    }
}
