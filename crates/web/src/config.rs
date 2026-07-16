use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub cookie_key: String,
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    pub static_dir: Option<String>,
    pub oidc: Option<OidcConfig>,
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
            .add_source(config::Environment::with_prefix("HEALTH").separator("__"))
            .build()?
            .try_deserialize()?;
        Ok(cfg)
    }
}
