use std::fs;
use std::fs::File;
use std::io::Write;

use anyhow::anyhow;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::enums::GameDifficulty;

const MAX_ENTRIES_PER_BOARD: usize = 25;

#[derive(Clone, Deserialize, Serialize)]
pub struct Entry {
    pub player_name: String,

    #[serde(with = "as_df_date")]
    pub date: DateTime<FixedOffset>,
    pub time: f64,
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct LeaderBoard {
    pub entries: Vec<Entry>,
}

impl LeaderBoard {
    pub fn add(&mut self, player_name: &str, time: f64) {
        self.entries.push(Entry {
            player_name: player_name.to_string(),
            date: Local::now().fixed_offset(),
            time,
        });
        self.sort_and_trim();
    }

    pub fn sort_and_trim(&mut self) {
        self.entries.sort_by(|a, b| a.time.total_cmp(&b.time));
        if self.entries.len() > MAX_ENTRIES_PER_BOARD {
            self.entries = self.entries[0..MAX_ENTRIES_PER_BOARD].to_vec();
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct LeaderBoards {
    pub beginner: LeaderBoard,
    pub intermediate: LeaderBoard,
    pub expert: LeaderBoard,
}

impl LeaderBoards {
    #[allow(dead_code)]
    pub fn leaderboard_for_level(&self, level: GameDifficulty) -> LeaderBoard {
        match level {
            GameDifficulty::Beginner => &self.beginner,
            GameDifficulty::Intermediate => &self.intermediate,
            GameDifficulty::Expert => &self.expert,
        }
        .clone()
    }

    pub fn add(&mut self, level: GameDifficulty, player_name: &str, time: f64) {
        match level {
            GameDifficulty::Beginner => &mut self.beginner,
            GameDifficulty::Intermediate => &mut self.intermediate,
            GameDifficulty::Expert => &mut self.expert,
        }
        .add(player_name, time);
    }

    pub fn load_from_userhome() -> anyhow::Result<Self> {
        let config_file_path = dirs::home_dir()
            .unwrap()
            .join(".apoapsys/minesofrust-leaderboard.toml");
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
        let config_file_path = apoapsys_config_dir.join("minesofrust-leaderboard.toml");
        let mut f = File::create(config_file_path).expect("Failed to create config file");
        f.write_all(toml_str.as_bytes())
            .expect("Failed to write to config file");
        println!("{}", toml_str);
    }
}

#[test]
fn test_leaderboards() -> Result<(), anyhow::Error> {
    let mut leaderboard = LeaderBoards::default();
    assert_eq!(
        leaderboard
            .leaderboard_for_level(GameDifficulty::Beginner)
            .entries
            .len(),
        0
    );

    leaderboard.add(GameDifficulty::Beginner, "Player 1", 100.0);
    assert_eq!(
        leaderboard
            .leaderboard_for_level(GameDifficulty::Beginner)
            .entries
            .len(),
        1
    );
    leaderboard.add(GameDifficulty::Beginner, "Player 2", 300.0);
    leaderboard.add(GameDifficulty::Beginner, "Player 3", 200.0);
    assert_eq!(
        leaderboard
            .leaderboard_for_level(GameDifficulty::Beginner)
            .entries
            .len(),
        3
    );
    assert_eq!(leaderboard.beginner.entries[1].player_name, "Player 3");

    (0..MAX_ENTRIES_PER_BOARD + 10).for_each(|_| {
        leaderboard.add(GameDifficulty::Beginner, "Player 2", 300.0);
    });
    assert_eq!(
        leaderboard
            .leaderboard_for_level(GameDifficulty::Beginner)
            .entries
            .len(),
        MAX_ENTRIES_PER_BOARD
    );

    // leaderboard.save_to_userhome();
    // let lb_reloaded = LeaderBoards::load_from_userhome()?;
    // assert_eq!(
    //     lb_reloaded
    //         .leaderboard_for_level(GameDifficulty::Beginner)
    //         .entries
    //         .len(),
    //     leaderboard
    //         .leaderboard_for_level(GameDifficulty::Beginner)
    //         .entries
    //         .len(),
    // );

    Ok(())
}

pub mod as_df_date {
    use chrono::{DateTime, FixedOffset, Local};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f %z";

    pub fn serialize<S>(date: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(Local::now().fixed_offset())
        } else {
            DateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
        }
    }
}
