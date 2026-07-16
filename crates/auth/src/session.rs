use cookie::{Cookie, CookieJar, Key, SameSite};
use thiserror::Error;
use time::Duration;

const SESSION_COOKIE_NAME: &str = "health_session";

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("failed to parse session cookie")]
    CookieParse(#[source] cookie::ParseError),

    #[error("invalid signature")]
    InvalidSignature,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub user_id: String,
}

/// Create a signed session cookie.
///
/// # Errors
///
/// Returns [`SessionError::CookieParse`] if the cookie cannot be created.
pub fn create_session_cookie(
    data: &SessionData,
    key: &Key,
    max_age: Duration,
) -> Result<String, SessionError> {
    let mut jar = CookieJar::new();
    let cookie = Cookie::build((SESSION_COOKIE_NAME, data.user_id.clone()))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(max_age)
        .build();

    jar.signed_mut(key).add(cookie);

    Ok(jar
        .get(SESSION_COOKIE_NAME)
        .map(ToString::to_string)
        .unwrap_or_default())
}

/// Parse and validate a signed session cookie from a cookie header.
///
/// # Errors
///
/// - [`SessionError::CookieParse`] if the cookie header cannot be parsed.
/// - [`SessionError::InvalidSignature`] if the cookie signature is invalid.
pub fn parse_session_cookie(cookie_header: &str, key: &Key) -> Result<SessionData, SessionError> {
    let cookie = Cookie::parse_encoded(cookie_header).map_err(SessionError::CookieParse)?;

    if cookie.name() != SESSION_COOKIE_NAME {
        return Err(SessionError::InvalidSignature);
    }

    let mut jar = CookieJar::new();
    jar.add(cookie.into_owned());

    let signed_cookie = jar
        .signed(key)
        .get(SESSION_COOKIE_NAME)
        .ok_or(SessionError::InvalidSignature)?;

    let user_id = signed_cookie.value().to_owned();

    Ok(SessionData { user_id })
}

/// Create a cookie string that deletes the session cookie.
#[must_use]
pub fn delete_session_cookie(key: &Key) -> String {
    let mut jar = CookieJar::new();
    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(Duration::ZERO)
        .build();

    jar.signed_mut(key).add(cookie);

    jar.get(SESSION_COOKIE_NAME)
        .map(ToString::to_string)
        .unwrap_or_default()
}
