use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize, Clone)]
pub enum VisualTheme {
    Light,
    Dark,
}

impl VisualTheme {
    pub fn as_str(&self) -> &'static str {
        match *self {
            VisualTheme::Dark => "Dark",
            VisualTheme::Light => "Light",
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Deserialize, Serialize)]
pub enum GameState {
    NotStarted,
    Playing,
    EndedLoss,
    EndedWin,
    Paused,
}

impl GameState {
    pub fn game_ended(&self) -> bool {
        *self == GameState::EndedLoss || *self == GameState::EndedWin
    }
}

#[derive(Eq, PartialEq, Clone, Deserialize, Serialize)]
pub enum GameDifficulty {
    Beginner,
    Intermediate,
    Expert,
    // Custom,
}

impl GameDifficulty {
    pub fn as_str(&self) -> &'static str {
        match *self {
            GameDifficulty::Beginner => "Beginner",
            GameDifficulty::Intermediate => "Intermediate",
            GameDifficulty::Expert => "Expert",
            // GameDifficulty::Custom => "Custom",
        }
    }
}
