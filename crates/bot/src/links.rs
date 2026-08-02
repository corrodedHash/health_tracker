//! Local persistence of the matrix-user → API-token mapping.
//!
//! The web server never sees this mapping — it only ever returns a token
//! bound to the confirming user. The bot stores `matrix_user_id → token`
//! in a small TOML file (mirroring the `matrix_auth` session-file pattern)
//! so the mapping survives restarts.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
struct LinksFile {
    links: HashMap<String, String>,
}

/// Matrix user id → API token mapping, persisted to a TOML file.
#[derive(Debug)]
pub struct LinksStore {
    path: PathBuf,
    links: HashMap<String, String>,
}

impl LinksStore {
    /// Load the mapping from `path`, treating a missing or corrupt file
    /// as an empty store.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        let links = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| toml::from_str::<LinksFile>(&raw).ok())
            .map_or_else(HashMap::new, |file| file.links);
        Self {
            path: path.to_owned(),
            links,
        }
    }

    /// The API token linked to `matrix_user_id`, if any.
    #[must_use]
    pub fn token_for(&self, matrix_user_id: &str) -> Option<&str> {
        self.links.get(matrix_user_id).map(String::as_str)
    }

    /// Link an API token to `matrix_user_id` and persist.
    ///
    /// # Errors
    /// Returns an error if the TOML file cannot be written.
    pub fn set_token(&mut self, matrix_user_id: &str, token: &str) -> anyhow::Result<()> {
        self.links
            .insert(matrix_user_id.to_owned(), token.to_owned());
        self.save()
    }

    /// Drop the token for `matrix_user_id` (e.g. after an unauthorized
    /// upload) and persist.
    ///
    /// # Errors
    /// Returns an error if the TOML file cannot be written.
    pub fn remove_token(&mut self, matrix_user_id: &str) -> anyhow::Result<()> {
        self.links.remove(matrix_user_id);
        self.save()
    }

    fn save(&self) -> anyhow::Result<()> {
        let file = LinksFile {
            links: self.links.clone(),
        };
        std::fs::write(&self.path, toml::to_string(&file)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use super::LinksStore;

    #[test]
    fn load_missing_file_is_empty() {
        let store = LinksStore::load(std::path::Path::new("/nonexistent/links.toml"));
        assert!(store.token_for("@alice:example.org").is_none());
    }

    #[test]
    fn set_and_remove_token_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("links.toml");

        let mut store = LinksStore::load(&path);
        store.set_token("@alice:example.org", "token-a").unwrap();
        store.set_token("@bob:example.org", "token-b").unwrap();
        assert_eq!(store.token_for("@alice:example.org"), Some("token-a"));
        assert_eq!(store.token_for("@bob:example.org"), Some("token-b"));
        assert!(store.token_for("@carol:example.org").is_none());

        let reloaded = LinksStore::load(&path);
        assert_eq!(reloaded.token_for("@alice:example.org"), Some("token-a"));

        let mut store = reloaded;
        store.remove_token("@alice:example.org").unwrap();
        assert!(store.token_for("@alice:example.org").is_none());
        assert_eq!(store.token_for("@bob:example.org"), Some("token-b"));
    }
}
