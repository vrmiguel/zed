use anyhow::{bail, Result};
use url::Url;

/// Test function copied from FetchSlashCommand
fn validate_url(input: &str) -> Result<String> {
    // Try to parse as is first
    let parsed_url = Url::parse(input);

    // If the URL is valid as-is, return it
    if let Ok(url) = parsed_url {
        // Only allow http and https schemes for security
        if url.scheme() == "http" || url.scheme() == "https" {
            return Ok(url.to_string());
        } else {
            bail!("Only http:// and https:// URLs are supported");
        }
    }

    // Try with https:// prefix if the original parsing failed
    if !input.starts_with("https://") && !input.starts_with("http://") {
        let with_https = format!("https://{input}");
        match Url::parse(&with_https) {
            Ok(url) => {
                // Verify the URL has a valid host
                if url.host().is_some() {
                    return Ok(url.to_string());
                }
                bail!("Invalid URL: Missing host");
            }
            Err(e) => bail!("Invalid URL: {}", e),
        }
    } else {
        // If it already had a scheme but failed to parse, it's invalid
        bail!("Invalid URL format: {}", input);
    }
}

fn test_url(url: &str, expected_result: bool, expected_output: Option<&str>) {
    print!("Testing '{}': ", url);
    
    match validate_url(url) {
        Ok(result) => {
            if expected_result {
                println!("✅ Valid: {}", result);
                if let Some(expected) = expected_output {
                    if result != expected {
                        println!("   ⚠️ Warning: Expected '{}' but got '{}'", expected, result);
                    }
                }
            } else {
                println!("❌ Expected invalid, but was valid: {}", result);
            }
        },
        Err(e) => {
            if expected_result {
                println!("❌ Expected valid, but was invalid: {}", e);
            } else {
                println!("✅ Invalid as expected: {}", e);
            }
        }
    }
}

fn main() {
    println!("\n=== Testing Valid URLs ===");
    // Valid URLs with schemes
    test_url("https://example.com", true, Some("https://example.com/"));
    test_url("http://example.com", true, Some("http://example.com/"));
    test_url("https://example.com/path?query=value#fragment", true, Some("https://example.com/path?query=value#fragment"));
    test_url("http://user:password@example.com:8080", true, Some("http://user:password@example.com:8080/"));
    test_url("https://subdomain.example.co.uk", true, Some("https://subdomain.example.co.uk/"));
    
    // Valid URLs without schemes (should be prefixed with https://)
    println!("\n=== Testing URLs without schemes ===");
    test_url("example.com", true, Some("https://example.com/"));
    test_url("www.example.com", true, Some("https://www.example.com/"));
    test_url("example.com/path", true, Some("https://example.com/path"));
    test_url("subdomain.example.co.uk", true, Some("https://subdomain.example.co.uk/"));
    
    // Invalid URLs with disallowed schemes
    println!("\n=== Testing URLs with disallowed schemes ===");
    test_url("javascript:alert(1)", false, None);
    test_url("data:text/html,<script>alert(1)</script>", false, None);
    test_url("file:///etc/passwd", false, None);
    test_url("ftp://example.com", false, None);
    
    // Malformed or potentially malicious URLs
    println!("\n=== Testing Malformed URLs ===");
    test_url("http://", false, None);
    test_url("https://", false, None);
    test_url("http:/example.com", false, None);
    test_url("https:/example.com", false, None);
    test_url("http:\\\\example.com", false, None);
    test_url("//example.com", false, None);
    test_url("javascript://example.com/%0Aalert(1)", false, None);
    
    // Edge cases
    println!("\n=== Testing Edge Cases ===");
    test_url("", false, None);
    test_url(" ", false, None);
    test_url("https://example.com with spaces", false, None);
    test_url("https://üñîçøðé.com", true, Some("https://xn--9ca9hlb7d.com/"));
    test_url("üñîçøðé.com", true, Some("https://xn--9ca9hlb7d.com/"));
    
    // IP addresses
    println!("\n=== Testing IP Addresses ===");
    test_url("http://127.0.0.1", true, Some("http://127.0.0.1/"));
    test_url("https://127.0.0.1:8080", true, Some("https://127.0.0.1:8080/"));
    test_url("127.0.0.1", true, Some("https://127.0.0.1/"));
    
    // Potential bypass attempts
    println!("\n=== Testing Potential Bypass Attempts ===");
    test_url("http://example.com@evil.com", true, Some("http://example.com@evil.com/"));
    test_url("https:/\\/example.com", false, None);
    test_url("java\nscript:alert(1)", false, None);
    test_url("https://example.com%20javascript:alert(1)", false, None);
    test_url("http://evil.com/%%30%30", true, Some("http://evil.com/%00"));
}