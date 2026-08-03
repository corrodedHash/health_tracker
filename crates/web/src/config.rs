use serde::Deserialize;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub cookie_key: String,
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    pub static_dir: Option<String>,
    pub oidc: Option<OidcConfig>,
    pub frontend_url: Option<String>,
    /// Public origin used to build browser-facing URLs (e.g. the account
    /// link page). Falls back to `frontend_url`, then to the request Host.
    pub public_base_url: Option<String>,
    #[serde(default)]
    pub dev_auto_login: bool,
    #[serde(default = "default_cookie_secure")]
    pub cookie_secure: bool,
    /// How long a bot-initiated account link stays valid before it expires.
    #[serde(default = "default_link_ttl_minutes")]
    pub link_ttl_minutes: i64,
}

const fn default_cookie_secure() -> bool {
    true
}

const fn default_link_ttl_minutes() -> i64 {
    5
}

fn default_listen_addr() -> String {
    "0.0.0.0:3000".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::File::with_name("config/local").required(false))
            .add_source(config::Environment::with_prefix("HEALTH").separator("__"))
            .build()?
            .try_deserialize()?;
        Ok(cfg)
    }
}
