use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for public routes
    if req.uri().path().contains("/public") || 
       req.uri().path() == "/health" ||
       req.uri().path() == "/ws" ||
       req.uri().path() == "/metrics" {
        return Ok(next.run(req).await);
    }

    // Extract token from Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // Validate token
    // TODO: Get secret from config
    let secret = "super-secret-jwt-key-change-in-production";
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    
    match decode::<Claims>(token, &decoding_key, &Validation::default()) {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}



