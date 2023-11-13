use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

use anyhow::Context;
use derive_getters::Getters;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Getters)]
#[serde(default)]
pub struct Config {
    default_clipboard: String,
    clipboards: HashMap<String, ClipboardConfig>,
    }
impl Config {

    pub fn from_toml(content: &str) -> Result<Config, anyhow::Error> {
        let config=toml::from_str(content).context("Unable to parse the configuration")?;

        Ok(config)
        }
    pub fn load() -> Result<Config, anyhow::Error> {
        let mut local_config_path=std::env::current_exe().unwrap();
        local_config_path.pop();
        local_config_path.push("config.toml");

        let mut user_level_config_path=dirs::config_dir().unwrap();
        user_level_config_path.extend(&["clipshare", "config.toml"]);

        let config_paths: Vec<PathBuf>=[
            local_config_path,
            user_level_config_path,
            ]
        .into_iter()
        .filter(|path| path.exists() && path.is_file())
        .collect();

        if let Some(path)=config_paths.into_iter().next() {
            let content=fs::read_to_string(&path).with_context(|| format!("Unable to read config from {path:?}"))?;
            let config=Config::from_toml(&content).with_context(|| format!("Unable to parse the config from {path:?}"))?;

            return Ok(config);
            }

        Ok(Config::default())
        }

    }
impl Default for Config {

    fn default() -> Config {
        let mut clipboards=HashMap::new();
        clipboards.insert("Primary".to_string(), ClipboardConfig::default());

        Config {
            default_clipboard: String::from("Primary"),
            clipboards
            }
        }
    }

#[derive(Serialize, Deserialize, Getters)]
#[serde(default)]
pub struct ClipboardConfig {
    host: String,
    password: String,
    copy_hotkey: String,
    paste_hotkey: String,
    sync_copy_hotkey: String,
    sync_paste_hotkey: String,
    }
impl Default for ClipboardConfig {

    fn default() -> ClipboardConfig {
        ClipboardConfig {
            host: String::from("https://clipshare.rastislavkish.xyz"),
            password: String::from("DefaultPassword"),
            copy_hotkey: String::new(),
            paste_hotkey: String::new(),
            sync_copy_hotkey: String::new(),
            sync_paste_hotkey: String::new(),
            }
        }
    }

