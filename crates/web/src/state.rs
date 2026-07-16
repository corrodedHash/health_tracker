use std::sync::Arc;

use sqlx::PgPool;

use health_auth::oidc::OidcClientBundle;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub cookie_key: cookie::Key,
    pub oidc_bundle: Option<Arc<OidcClientBundle>>,
}
