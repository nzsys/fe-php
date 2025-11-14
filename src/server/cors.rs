use hyper::{Request, Response, Method, StatusCode, header};
use std::collections::HashSet;

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: HashSet<String>,
    pub allowed_methods: HashSet<String>,
    pub allowed_headers: HashSet<String>,
    pub exposed_headers: Vec<String>,
    pub max_age: u64,
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: ["*".to_string()].iter().cloned().collect(),
            allowed_methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
                .iter().map(|s| s.to_string()).collect(),
            allowed_headers: ["Content-Type", "Authorization"]
                .iter().map(|s| s.to_string()).collect(),
            exposed_headers: Vec::new(),
            max_age: 3600,
            allow_credentials: false,
        }
    }
}

/// CORS middleware
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }

    /// Check if origin is allowed
    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.config.allowed_origins.contains("*")
            || self.config.allowed_origins.contains(origin)
    }

    /// Handle preflight OPTIONS request
    pub fn handle_preflight<T>(&self, req: &Request<T>) -> Option<Response<String>> {
        if req.method() != Method::OPTIONS {
            return None;
        }

        let origin = req.headers().get(header::ORIGIN)
            .and_then(|h| h.to_str().ok())?;

        if !self.is_origin_allowed(origin) {
            return Some(Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(String::new())
                .unwrap());
        }

        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT);

        response = response
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .header(header::ACCESS_CONTROL_ALLOW_METHODS,
                self.config.allowed_methods.iter().cloned().collect::<Vec<_>>().join(", "))
            .header(header::ACCESS_CONTROL_ALLOW_HEADERS,
                self.config.allowed_headers.iter().cloned().collect::<Vec<_>>().join(", "))
            .header(header::ACCESS_CONTROL_MAX_AGE, self.config.max_age.to_string());

        if self.config.allow_credentials {
            response = response.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        Some(response.body(String::new()).unwrap())
    }

    /// Add CORS headers to response
    pub fn add_cors_headers<T>(&self, response: &mut Response<T>, origin: Option<&str>) {
        if let Some(origin) = origin {
            if self.is_origin_allowed(origin) {
                response.headers_mut().insert(
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    origin.parse().unwrap(),
                );

                if !self.config.exposed_headers.is_empty() {
                    response.headers_mut().insert(
                        header::ACCESS_CONTROL_EXPOSE_HEADERS,
                        self.config.exposed_headers.join(", ").parse().unwrap(),
                    );
                }

                if self.config.allow_credentials {
                    response.headers_mut().insert(
                        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        "true".parse().unwrap(),
                    );
                }
            }
        }
    }
}
