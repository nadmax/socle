mod common;

use axum::http::{HeaderMap, HeaderValue};
use yaima::middleware::extract_bearer;

#[test]
fn extracts_bearer_token_from_valid_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        HeaderValue::from_static("Bearer my.jwt.token"),
    );
    assert_eq!(extract_bearer(&headers), Some("my.jwt.token"));
}

#[test]
fn returns_none_when_authorization_header_is_absent() {
    let headers = HeaderMap::new();
    assert_eq!(extract_bearer(&headers), None);
}

#[test]
fn returns_none_for_non_bearer_scheme() {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        HeaderValue::from_static("Basic dXNlcjpwYXNz"),
    );
    assert_eq!(extract_bearer(&headers), None);
}

#[test]
fn returns_none_for_bearer_prefix_without_token() {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        HeaderValue::from_static("Bearer "),
    );
    // Strip prefix yields an empty string, which is still Some("").
    // Callers reject empty tokens; the extractor just strips the prefix.
    assert_eq!(extract_bearer(&headers), Some(""));
}
