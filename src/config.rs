use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub maintainer_id: teloxide::types::UserId,
    pub authorized_users: Vec<AuthorizedUser>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthorizedUser {
    pub name: String,
    pub id: u64,
}

impl std::fmt::Display for AuthorizedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

pub struct ConfigStorage {
    file_path: String,
    cfg: Config,
}

impl ConfigStorage {
    pub fn new(file_path: &str) -> ConfigStorage {
        let file_content = std::fs::read_to_string(file_path).expect("Failed to read config file.");
        let cfg: Config =
            serde_json::from_str(&file_content).expect("Failed to parse config file.");

        ConfigStorage {
            file_path: file_path.to_string(),
            cfg,
        }
    }

    pub fn get_config(&self) -> &Config {
        &self.cfg
    }

    pub fn get_config_mut(&mut self) -> &mut Config {
        &mut self.cfg
    }

    pub async fn save(&self) -> std::io::Result<()> {
        tokio::fs::write(
            &self.file_path,
            serde_json::to_string_pretty::<Config>(&self.cfg).unwrap(),
        )
        .await
    }
}
