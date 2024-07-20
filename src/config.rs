use std::{fs, io::{stdout, stderr}, path::{Path, PathBuf}, process};

use anyhow::Result;
use colored::Colorize;
use serde::{Serialize, Deserialize};
use toml;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub items: Vec<ConfigItem>
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigItem {
    CommandItem(ConfigCommandItem)
}

#[derive(Serialize, Deserialize)]
pub struct ConfigCommandItem {
    pub name: String,
    pub current: String,
    pub states: Vec<ConfigCommandItemState>
}

#[derive(Serialize, Deserialize)]
pub struct ConfigCommandItemState {
    pub name: String,
    pub command: String
}

impl ConfigItem {
    pub fn get_name(&self) -> &str {
        match self {
            ConfigItem::CommandItem(item) => &item.name
        }
    }

    pub fn get_current_state(&self) -> &str {
        match self {
            ConfigItem::CommandItem(item) => &item.current
        }
    }

    pub fn get_state_names(&self) -> Vec<&str> {
        match self {
            ConfigItem::CommandItem(item) => item.states
                .iter()
                .map(|state| state.name.as_str())
                .collect()
        }
    }

    pub fn set_current_state(&mut self, new_state: String) -> Result<()> {
        let item_name = self.get_name().to_string();

        match self {
            ConfigItem::CommandItem(item) => {
                item.current = new_state.clone();
                println!("Switching {} => {}", item_name.cyan(), new_state.yellow());

                if let Some(state) = item.states.iter().find(|state| state.name == *new_state) {
                    let ConfigCommandItemState { command, .. } = state;

                    println!("Running {} {}", "$".purple().bold(), command.purple());

                    if cfg!(target_os = "windows") {
                        process::Command::new("cmd")
                            .args(["/C", command])
                            .stdout(stdout())
                            .stderr(stderr())
                            .output()
                    } else {
                        process::Command::new("sh")
                            .args(["-c", command])
                            .stdout(stdout())
                            .stderr(stderr())
                            .output()
                    }?;
                }
            }
        }

        Ok(())
    }

    pub fn get_type_string(&self) -> String {
        match self {
            ConfigItem::CommandItem(_) => {
                "Command".to_string()
            }
        }
    }

    pub fn to_string(&self) -> String {
        let name = self.get_name();
        let type_string = self.get_type_string();
        let current_state = self.get_current_state();
        let state_names = self.get_state_names();

        format!(
            "{} [{}]\n{}",
            name.cyan(),
            type_string,
            state_names
                .iter()
                .map(|name| format!(
                    "{} {}",
                    if *name == current_state { "*".green().to_string() } else { " ".to_string() },
                    name.yellow()
                ))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

impl From<&ConfigItem> for String {
    fn from(item: &ConfigItem) -> String {
        item.to_string()
    }
}

pub struct ConfigManager<'a> {
    path: &'a Path,
    file_path: PathBuf,

    pub config: Config
}

impl<'a> ConfigManager<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self {
            path,
            file_path: path.join("config.toml"),
            config: Self::get_default_config()
        }
    }

    pub fn get_default_config() -> Config {
        Config {
            items: vec! []
        }
    }

    pub fn read(&mut self) -> Result<()> {
        fs::create_dir_all(&self.path)?;

        if self.file_path.exists() {
            let config_str = fs::read_to_string(&self.file_path)?;
            self.config = toml::from_str::<Config>(&config_str)?;
        }
        else {
            self.write()?;
        }

        Ok(())
    }

    pub fn write(&self) -> Result<()> {
        let config_str = toml::to_string_pretty(&self.config)?;
        fs::write(&self.file_path, config_str)?;

        Ok(())
    }
}