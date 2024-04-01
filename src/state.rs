use std::fs;
use std::fs::File;
use std::io::Write;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::constants::*;
use crate::enums::*;

#[derive(Clone, Deserialize, Serialize)]
pub struct GameSettings {
    pub width: u32,
    pub height: u32,
    pub num_mines: u32,
    pub use_numerals: bool,
    pub ui_width: f32,
    pub ui_height: f32,
}

impl GameSettings {
    pub fn beginner() -> Self {
        GameSettings {
            width: DEFAULT_BEGINNER_WIDTH,
            height: DEFAULT_BEGINNER_HEIGHT,
            num_mines: DEFAULT_BEGINNER_NUM_MINES,
            use_numerals: true,
            ui_width: DEFAULT_BEGINNER_UI_WIDTH,
            ui_height: DEFAULT_BEGINNER_UI_HEIGHT,
        }
    }

    pub fn intermediate() -> Self {
        GameSettings {
            width: DEFAULT_INTERMEDIATE_WIDTH,
            height: DEFAULT_INTERMEDIATE_HEIGHT,
            num_mines: DEFAULT_INTERMEDIATE_NUM_MINES,
            use_numerals: true,
            ui_width: DEFAULT_INTERMEDIATE_UI_WIDTH,
            ui_height: DEFAULT_INTERMEDIATE_UI_HEIGHT,
        }
    }

    pub fn expert() -> Self {
        GameSettings {
            width: DEFAULT_EXPERT_WIDTH,
            height: DEFAULT_EXPERT_HEIGHT,
            num_mines: DEFAULT_EXPERT_NUM_MINES,
            use_numerals: true,
            ui_width: DEFAULT_EXPERT_UI_WIDTH,
            ui_height: DEFAULT_EXPERT_UI_HEIGHT,
        }
    }

    pub fn settings_for_difficulty(difficulty: &GameDifficulty) -> Self {
        match difficulty {
            GameDifficulty::Beginner => GameSettings::beginner(),
            GameDifficulty::Intermediate => GameSettings::intermediate(),
            GameDifficulty::Expert => GameSettings::expert(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AppState {
    pub difficulty: GameDifficulty,
    pub left_click_chord: bool,
    pub theme: VisualTheme,
    pub fog_of_war: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            difficulty: GameDifficulty::Intermediate,
            left_click_chord: false,
            theme: VisualTheme::Dark,
            fog_of_war: false,
        }
    }
}

impl AppState {
    pub fn load_from_userhome() -> Result<Self> {
        let config_file_path = dirs::home_dir().unwrap().join(".apoapsys/minesofrust.toml");
        if config_file_path.exists() {
            println!(
                "Window state config file exists at path: {:?}",
                config_file_path
            );
            let t = std::fs::read_to_string(config_file_path)?;
            Ok(toml::from_str(&t)?)
        } else {
            println!("Window state config file does not exist. Will be created on exit");
            Err(anyhow!("Config file does not exist"))
        }
    }

    pub fn save_to_userhome(&self) {
        let toml_str = toml::to_string(&self).unwrap();
        let apoapsys_config_dir = dirs::home_dir().unwrap().join(".apoapsys/");
        if !apoapsys_config_dir.exists() {
            fs::create_dir(&apoapsys_config_dir).expect("Failed to create config directory");
        }
        let config_file_path = apoapsys_config_dir.join("minesofrust.toml");
        let mut f = File::create(config_file_path).expect("Failed to create config file");
        f.write_all(toml_str.as_bytes())
            .expect("Failed to write to config file");
        println!("{}", toml_str);
    }
}
