#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::process;
use std::time::SystemTime;

use anyhow::Result;
use eframe::{egui, glow, Theme};
use egui::{
    Color32, Key, KeyboardShortcut, Modifiers, Pos2, RichText, Stroke, Vec2, ViewportCommand,
    Visuals,
};
use egui_extras::install_image_loaders;
use itertools::iproduct;

use crate::constants;
use crate::enums::*;
use crate::minesweeper::*;
use crate::state::*;
use crate::toggle::*;
use serde::{Deserialize, Serialize};

use crate::leader::LeaderBoards;

/// Settings as 'true' will allow the window to be resized and will print the dimensions to the console.
const DBG_WINDOW_RESIZABLE: bool = false;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct PlayEntry {
    #[allow(dead_code)]
    coord: Coordinate,
    play_type: RevealType,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct PlayList {
    pub list: Vec<PlayEntry>,
}

impl PlayList {
    pub fn push(&mut self, entry: PlayEntry) {
        self.list.push(entry);
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn clicks(&self) -> u32 {
        self.list.len() as u32
    }

    pub fn reveals(&self) -> u32 {
        self.list
            .iter()
            .map(|e| match e.play_type {
                RevealType::Reveal | RevealType::RevealChord => 1,
                _ => 0,
            })
            .collect::<Vec<u32>>()
            .iter()
            .sum()
    }

    pub fn flagged(&self) -> u32 {
        self.list
            .iter()
            .map(|e| match e.play_type {
                RevealType::Flag => 1,
                _ => 0,
            })
            .collect::<Vec<u32>>()
            .iter()
            .sum()
    }

    pub fn chords(&self) -> u32 {
        self.list
            .iter()
            .map(|e| match e.play_type {
                RevealType::Chord | RevealType::RevealChord => 1,
                _ => 0,
            })
            .collect::<Vec<u32>>()
            .iter()
            .sum()
    }
}

fn now() -> f64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs_f64(),
        Err(_) => 0.0,
    }
}

#[derive(Clone)]
pub struct MinesOfRustApp {
    gameboard: GameBoard,
    state: AppState,
    image_loaders_installed: bool,
    detonated_on: Option<Coordinate>,
    game_state: GameState,
    game_started: f64,
    game_finished: f64,
    game_settings: GameSettings,
    leaderboards: LeaderBoards,
    leaderboard_visible: bool,
    gamestats_visible: bool,
    plays: PlayList,
    wins: u32,
    losses: u32,
}

#[cfg(not(target_arch = "wasm32"))]
impl MinesOfRustApp {
    pub fn load_from_persistence() -> MinesOfRustApp {
        let state = AppState::load_from_userhome().unwrap_or_default();
        let leaderboards = LeaderBoards::load_from_userhome().unwrap_or_default();
        let settings = GameSettings::settings_for_difficulty(&state.difficulty);

        MinesOfRustApp {
            gameboard: GameBoard::new(settings.width, settings.height),
            state,
            image_loaders_installed: false,
            detonated_on: None,
            game_state: GameState::NotStarted,
            game_started: 0.0,
            game_finished: 0.0,
            game_settings: settings,
            leaderboards,
            leaderboard_visible: false,
            gamestats_visible: false,
            plays: PlayList::default(),
            wins: 0,
            losses: 0,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl MinesOfRustApp {
    pub fn load_from_persistence() -> MinesOfRustApp {
        let settings = GameSettings::beginner();
        let state = AppState::default();
        let leaderboards = LeaderBoards::default();

        MinesOfRustApp {
            gameboard: GameBoard::new(settings.width, settings.height),
            state,
            image_loaders_installed: false,
            detonated_on: None,
            game_state: GameState::NotStarted,
            game_started: 0.0,
            game_finished: 0.0,
            game_settings: settings,
            leaderboards,
            leaderboard_visible: false,
            gamestats_visible: false,
            plays: PlayList::default(),
            wins: 0,
            losses: 0,
        }
    }
}

impl eframe::App for MinesOfRustApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.on_update(ctx, frame).expect("Failed to update UI");
    }

    fn on_exit(&mut self, _gl: Option<&glow::Context>) {
        self.state.save_to_userhome();
        self.leaderboards.save_to_userhome();
    }
}

impl MinesOfRustApp {
    fn update_difficulty_settings(&mut self) {
        self.game_settings = match self.state.difficulty {
            GameDifficulty::Beginner => GameSettings::beginner(),
            GameDifficulty::Intermediate => GameSettings::intermediate(),
            GameDifficulty::Expert => GameSettings::expert(),
            // _ => unimplemented!(),
        };
    }

    fn reset_new_game(&mut self, ctx: &egui::Context) -> Result<(), Error> {
        self.gameboard = GameBoard::new(self.game_settings.width, self.game_settings.height);
        self.plays.clear();
        self.game_state = GameState::NotStarted;
        self.detonated_on = None;
        self.game_started = now();

        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2 {
            x: self.game_settings.ui_width,
            y: self.game_settings.ui_height,
        }));

        Ok(())
    }

    fn reset_existing_game(&mut self, _ctx: &egui::Context) -> Result<(), Error> {
        self.gameboard.reset_existing();

        self.plays.clear();
        self.game_state = GameState::NotStarted;
        self.game_started = now();

        Ok(())
    }

    fn start_game(&mut self, first_click: Coordinate) -> Result<(), Error> {
        println!(
            "Starting game with fist click at x={}, y={}",
            first_click.x, first_click.y
        );

        // Make sure we remove any previous mines
        //self.gameboard.reset();
        if !self.gameboard.is_populated {
            self.gameboard
                .populate_mines_around(self.game_settings.num_mines, Some(first_click))?;
        }

        self.game_started = now();
        self.game_state = GameState::Playing;

        if self.game_settings.use_numerals {
            self.gameboard.populate_numerals()?;
        }

        Ok(())
    }

    fn leaderboard_ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Leaderboard")
            .open(&mut self.leaderboard_visible)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| {
                egui::CollapsingHeader::new("Beginner")
                    .default_open(self.state.difficulty == GameDifficulty::Beginner)
                    .show(ui, |ui| {
                        egui::Grid::new("leaderboard")
                            .num_columns(2)
                            .spacing([50.0, 5.0])
                            .striped(true)
                            .show(ui, |ui| {
                                self.leaderboards.beginner.entries.iter().for_each(|e| {
                                    ui.label(&e.player_name);
                                    ui.label(format!("{:.2}", e.time));
                                    ui.label(format!("{}", e.date.format("%Y-%m-%d %H:%M")));
                                    ui.end_row();
                                });
                            });
                    });

                egui::CollapsingHeader::new("Intermediate")
                    .default_open(self.state.difficulty == GameDifficulty::Intermediate)
                    .show(ui, |ui| {
                        egui::Grid::new("leaderboard")
                            .num_columns(3)
                            .spacing([50.0, 5.0])
                            .striped(true)
                            .show(ui, |ui| {
                                self.leaderboards.intermediate.entries.iter().for_each(|e| {
                                    ui.label(&e.player_name);
                                    ui.label(format!("{:.2}", e.time));
                                    ui.label(format!("{}", e.date.format("%Y-%m-%d %H:%M")));
                                    ui.end_row();
                                });
                            });
                    });

                egui::CollapsingHeader::new("Expert")
                    .default_open(self.state.difficulty == GameDifficulty::Expert)
                    .show(ui, |ui| {
                        egui::Grid::new("leaderboard")
                            .num_columns(2)
                            .spacing([50.0, 5.0])
                            .striped(true)
                            .show(ui, |ui| {
                                self.leaderboards.expert.entries.iter().for_each(|e| {
                                    ui.label(&e.player_name);
                                    ui.label(format!("{:.2}", e.time));
                                    ui.label(format!("{}", e.date.format("%Y-%m-%d %H:%M")));
                                    ui.end_row();
                                });
                            });
                    });
            });
    }

    fn gamestats_ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Game Stats")
            .open(&mut self.gamestats_visible)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| {
                egui::Grid::new("leaderboard")
                    .num_columns(2)
                    .spacing([50.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Reveal Clicks:");
                        ui.label(format!("{}", self.plays.reveals()));
                        ui.end_row();

                        ui.label("Chord Clicks:");
                        ui.label(format!("{}", self.plays.chords()));
                        ui.end_row();

                        ui.label("Flag Clicks:");
                        ui.label(format!("{}", self.plays.flagged()));
                        ui.end_row();

                        ui.label("Total Clicks:");
                        ui.label(format!("{}", self.plays.clicks()));
                        ui.end_row();

                        let num_sqrs_worked =
                            self.gameboard.num_flags() + self.gameboard.num_revealed();
                        ui.label("Squares Revealed + Flagged:");
                        ui.label(format!("{}", num_sqrs_worked));
                        ui.end_row();

                        ui.label("Efficiency:");
                        if num_sqrs_worked > 0 {
                            ui.label(format!(
                                "{:.2}%",
                                (1.0 - self.plays.clicks() as f32 / num_sqrs_worked as f32) * 100.0
                            ));
                        }
                        ui.end_row();

                        ui.label("Session Wins:");
                        ui.label(format!(
                            "{} of {} games",
                            self.wins,
                            self.wins + self.losses
                        ));
                    });
            });
    }

    fn on_update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Result<(), Error> {
        if !self.image_loaders_installed {
            install_image_loaders(ctx);
            self.image_loaders_installed = true;
        }

        if self.leaderboard_visible {
            self.leaderboard_ui(ctx);
        }

        if self.gamestats_visible {
            self.gamestats_ui(ctx);
        }

        match self.state.theme {
            VisualTheme::Dark => ctx.set_visuals(Visuals::dark()),
            VisualTheme::Light => ctx.set_visuals(Visuals::light()),
        }

        if DBG_WINDOW_RESIZABLE {
            println!(
                "width: {}, height: {}",
                ctx.available_rect().width(),
                ctx.available_rect().height()
            );
        }

        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .min_height(50.0)
            .show(ctx, |ui| {
                // self.state.dark_mode = ui.visuals().dark_mode; // I don't like having this here.

                if ui.input_mut(|i| {
                    i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::N))
                }) {
                    println!("ctrl+n is pressed, resetting game");
                    self.reset_new_game(ctx).expect("Error building new game");
                }
                if ui.input_mut(|i| {
                    i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::R))
                }) {
                    println!("ctrl+r is pressed, resetting existing game");
                    self.reset_existing_game(ctx)
                        .expect("Error rebuilding game");
                }
                if ui.input_mut(|i| {
                    i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Q))
                }) {
                    println!("Boss can see screen. Ctrl+q is pressed, exiting");
                    process::exit(0);
                }
                if ui.input_mut(|i| {
                    i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::P))
                }) {
                    println!("Ctrl+q is pressed, toggling pause status");
                    self.toggle_pause_state();
                }

                ui.vertical_centered(|ui| {
                    let resp = self.face_ui(ui);
                    if resp.clicked_by(egui::PointerButton::Primary) {
                        self.reset_new_game(ctx).expect("Error building new game");
                    } else if resp.clicked_by(egui::PointerButton::Secondary) {
                        self.reset_existing_game(ctx)
                            .expect("Error building new game");
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if self.game_state != GameState::Paused {
                    self.game_board_ui(ui, !self.game_state.game_ended(), ctx.pointer_latest_pos());
                } else {
                    self.game_board_paused_ui(ui);
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(165.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    egui::Grid::new("app_options")
                        .num_columns(3)
                        .spacing([10.0, 50.0])
                        .striped(false)
                        .show(ui, |ui| {
                            //egui::CollapsingHeader::new("Options")
                            //    .default_open(false)
                            //    .show(ui, |ui| {
                            self.options_ui(ctx, ui);
                            //    });
                            self.status_ui(ui);
                        });

                    ui.horizontal_centered(|ui| {
                        if ui.button("Leaderboard").clicked() {
                            self.leaderboard_visible = true;
                        }
                        if ui.button("Game Stats").clicked() {
                            self.gamestats_visible = true;
                        }
                    });
                });
            });
        if self.game_state == GameState::Playing {
            ctx.request_repaint();
        }
        Ok(())
    }

    fn status_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("");
            let s = format!(
                "{} of {}",
                self.gameboard.num_flags(),
                self.gameboard.num_mines
            );
            ui.add(egui::Label::new(<String as Into<RichText>>::into(s).heading()).wrap(false));

            let s = if self.game_state == GameState::Playing
                && self.gameboard.is_loss_configuration()
            {
                self.game_state = GameState::EndedLoss;
                self.game_finished = now();
                self.losses += 1;
                "".to_string()
            } else if self.game_state == GameState::Playing && self.gameboard.is_win_configuration()
            {
                // You win!
                self.game_state = GameState::EndedWin;
                self.gameboard.flag_all_mines();
                self.game_finished = now();
                self.wins += 1;
                self.leaderboards.add(
                    self.state.difficulty.clone(),
                    &whoami::realname(), // Do this until I write a dialog asking for the real name
                    self.game_finished - self.game_started,
                );
                "".to_string()
            } else if self.game_state == GameState::Playing {
                format!("Time: {:.2}", now() - self.game_started)
            } else if self.game_state == GameState::Paused {
                format!("Time: {:.2}", self.game_started)
            } else if self.game_state.game_ended() {
                format!("Time: {:.2}", self.game_finished - self.game_started)
            } else {
                "".to_string()
            };

            ui.add(egui::Label::new(<String as Into<RichText>>::into(s).heading()).wrap(false));

            if self.game_state == GameState::Playing && ui.button("Pause").clicked() {
                self.pause_game();
            } else if self.game_state == GameState::Paused && ui.button("Resume").clicked() {
                self.resume_game();
            }
        });
    }

    fn toggle_pause_state(&mut self) {
        if self.game_state == GameState::Playing {
            self.pause_game();
        } else if self.game_state == GameState::Paused {
            self.resume_game();
        }
    }

    fn pause_game(&mut self) {
        self.game_state = GameState::Paused;
        self.game_started = now() - self.game_started;
    }

    fn resume_game(&mut self) {
        self.game_state = GameState::Playing;
        self.game_started = now() - self.game_started;
    }

    fn options_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::Grid::new("app_options")
            .num_columns(2)
            .spacing([5.0, 5.0])
            .min_row_height(30.0)
            .striped(false)
            .show(ui, |ui| {
                ui.label("Difficulty:");

                let cb = egui::ComboBox::new("GameDifficulty", "")
                    .width(0_f32)
                    .selected_text(self.state.difficulty.as_str());
                cb.show_ui(ui, |ui| {
                    let b = ui.selectable_value(
                        &mut self.state.difficulty,
                        GameDifficulty::Beginner,
                        "Beginner",
                    );
                    let i = ui.selectable_value(
                        &mut self.state.difficulty,
                        GameDifficulty::Intermediate,
                        "Intermediate",
                    );
                    let e = ui.selectable_value(
                        &mut self.state.difficulty,
                        GameDifficulty::Expert,
                        "Expert",
                    );
                    // I don't like this pattern:
                    if b.changed() || i.changed() || e.changed() {
                        self.update_difficulty_settings();
                        self.reset_new_game(ctx).expect("Failed to reset game");
                    }
                });
                ui.end_row();

                ui.label("Left Click Chords:");
                toggle_ui(ui, &mut self.state.left_click_chord);
                ui.end_row();

                ui.label("Fog of War:");
                toggle_ui(ui, &mut self.state.fog_of_war);
                ui.end_row();

                ui.label("Theme:");
                let cb = egui::ComboBox::new("VisualTheme", "")
                    .width(0_f32)
                    .selected_text(self.state.theme.as_str());
                cb.show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.state.theme, VisualTheme::Dark, "Dark");
                    ui.selectable_value(&mut self.state.theme, VisualTheme::Light, "Light");
                });
            });
    }

    /// Returns the first found Explosion in a list of cascaded play results
    fn first_losing_square_of_vec(play_result: &[PlayResult]) -> Option<Coordinate> {
        for r in play_result {
            if let PlayResult::Explosion(c) = r {
                return Some(c.clone());
            }
        }
        None
    }

    /// Returns the first found Explosion in either an explicit explosion or a cascaded play result
    fn first_losing_square(play_result: &PlayResult) -> Option<Coordinate> {
        match play_result {
            PlayResult::Explosion(c) => Some(c.clone()),
            PlayResult::CascadedReveal(r) => MinesOfRustApp::first_losing_square_of_vec(r),
            _ => None,
        }
    }

    fn game_board_paused_ui(&mut self, ui: &mut egui::Ui) {
        let desired_size = ui.spacing().interact_size.x
            * egui::vec2(
                self.game_settings.width as f32,
                self.game_settings.height as f32,
            );
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        ui.painter().rect(
            rect,
            1.0,
            constants::COLOR_REVEALED,
            Stroke::new(1.0, constants::COLOR_BORDER),
        );
    }

    fn game_board_ui(&mut self, ui: &mut egui::Ui, active: bool, pointer_pos: Option<Pos2>) {
        // This determines which square the mouse is over for fog-of-war mode
        let mouse_over_coord = if let Some(p) = pointer_pos {
            let n = ui.next_widget_position();
            let x = (p.x - ui.spacing().button_padding.x * 2.0) / ui.spacing().interact_size.x;
            let y = (p.y - n.y) / ui.spacing().interact_size.x;
            Coordinate {
                x: x.floor() as u32,
                y: y.floor() as u32,
            }
        } else {
            Coordinate { x: 9999, y: 9999 }
        };

        egui::Grid::new("process_grid_outputs")
            .spacing([0.0, 0.0])
            .striped(false)
            .show(ui, |ui| {
                iproduct!(0..self.gameboard.height, 0..self.gameboard.width).for_each(|(y, x)| {
                    let sqr = self
                        .gameboard
                        .get_square(x, y)
                        .expect("Error retrieving square");

                    let detonated = if let Some(c) = &self.detonated_on {
                        c.matches(x, y)
                    } else {
                        false
                    };

                    let resp = self.square_ui(
                        ui,
                        &sqr,
                        detonated,
                        mouse_over_coord.distance(&Coordinate { x, y }),
                    );
                    if resp.clicked() && self.game_state == GameState::NotStarted {
                        self.start_game(Coordinate { x, y })
                            .expect("Error starting game");
                    }

                    let play_type = if active
                        && resp.clicked_by(egui::PointerButton::Primary)
                        && !self.state.left_click_chord
                    {
                        Some(RevealType::Reveal)
                    } else if active
                        && resp.clicked_by(egui::PointerButton::Primary)
                        && self.state.left_click_chord
                    {
                        Some(RevealType::RevealChord)
                    } else if active && resp.clicked_by(egui::PointerButton::Middle) {
                        Some(RevealType::Chord)
                    } else if resp.clicked_by(egui::PointerButton::Secondary) && active {
                        Some(RevealType::Flag)
                    } else {
                        None
                    };

                    if let Some(p) = play_type {
                        self.plays.push(PlayEntry {
                            play_type: p.clone(),
                            coord: Coordinate { x, y },
                        });

                        if let Some(c) = MinesOfRustApp::first_losing_square(
                            &self
                                .gameboard
                                .play(x, y, p)
                                .expect("Failed to play desired move"),
                        ) {
                            println!("Detonated on {:?}", c);
                            self.detonated_on = Some(c.clone());
                        }
                    }

                    if x == self.gameboard.width - 1 {
                        ui.end_row();
                    }
                });
            });
    }

    fn face_ui(&self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = ui.spacing().interact_size.x * egui::vec2(1.4, 1.4);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if self.game_state == GameState::EndedLoss {
            egui::Image::new(egui::include_image!("../assets/loss.png")).paint_at(ui, rect);
        } else if self.game_state == GameState::EndedWin {
            egui::Image::new(egui::include_image!("../assets/win.png")).paint_at(ui, rect);
        } else {
            egui::Image::new(egui::include_image!("../assets/happy.png")).paint_at(ui, rect);
        }

        response
    }

    fn square_ui(
        &self,
        ui: &mut egui::Ui,
        sqr: &Square,
        is_detonated: bool,
        mouse_distance: f32,
    ) -> egui::Response {
        let opaque = mouse_distance > 1.5 && self.state.fog_of_war;

        let desired_size = (ui.spacing().interact_size.x) * egui::vec2(1.0, 1.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        let visuals_off = ui.style().interact_selectable(&response, false);
        let visuals_on = ui.style().interact_selectable(&response, true);

        let unrevealed_color = visuals_on.bg_fill; //constants::COLOR_UNREVEALED;
        let revealed_color = if is_detonated {
            constants::COLOR_DETONATED
        } else {
            visuals_off.bg_fill
        };
        let border_color = constants::COLOR_BORDER;
        let misflagged_color = constants::COLOR_MISFLAGGED;

        let opaque_color = Color32::from_rgba_unmultiplied(
            unrevealed_color.r(),
            unrevealed_color.g(),
            unrevealed_color.b(),
            if mouse_distance < 1.0 {
                0
            } else if mouse_distance < 3.0 {
                140
            } else {
                255
            },
        );

        ui.painter()
            .rect(rect, 0.0, revealed_color, Stroke::new(0.5, border_color));

        // Note: These are insufficient.
        // Playing
        //      Unrevealed
        //      Unrevealed Flagged
        //      Revealed numeral
        //      Revealed blank
        //      Unrevealed, Mouse down, left button
        //      Unrevealed, Mouse down, chord
        // Loss
        //      Unrevealed
        //      Unrevealed non-mined flagged
        //      Unrevealed mined flagged
        //      Revealed mined (losing play)
        //      Revealed mined (adjacent to losing play)
        //      Revealed numeral
        //      Revealed blank
        // Win
        //      Unrevealed
        //      Unrevealed flagged
        //      Revealed numeral
        //      Revealed blank
        if sqr.is_mine() && !sqr.is_flagged && self.game_state == GameState::EndedLoss {
            egui::Image::new(egui::include_image!("../assets/mine.png")).paint_at(ui, rect);
        } else if sqr.is_flagged && !sqr.is_mine() && self.game_state == GameState::EndedLoss {
            ui.painter()
                .rect(rect, 0.0, misflagged_color, Stroke::new(0.5, border_color));
            egui::Image::new(egui::include_image!("../assets/flag.png")).paint_at(ui, rect);
        } else if sqr.is_flagged {
            ui.painter()
                .rect(rect, 0.0, unrevealed_color, Stroke::new(0.5, border_color));
            egui::Image::new(egui::include_image!("../assets/flag.png")).paint_at(ui, rect);
        } else if sqr.is_revealed {
            match sqr.numeral {
                1 => egui::Image::new(egui::include_image!("../assets/1.png")).paint_at(ui, rect),
                2 => egui::Image::new(egui::include_image!("../assets/2.png")).paint_at(ui, rect),
                3 => egui::Image::new(egui::include_image!("../assets/3.png")).paint_at(ui, rect),
                4 => egui::Image::new(egui::include_image!("../assets/4.png")).paint_at(ui, rect),
                5 => egui::Image::new(egui::include_image!("../assets/5.png")).paint_at(ui, rect),
                6 => egui::Image::new(egui::include_image!("../assets/6.png")).paint_at(ui, rect),
                7 => egui::Image::new(egui::include_image!("../assets/7.png")).paint_at(ui, rect),
                8 => egui::Image::new(egui::include_image!("../assets/8.png")).paint_at(ui, rect),
                _ => {}
            };
        } else {
            ui.painter()
                .rect(rect, 0.0, unrevealed_color, Stroke::new(0.5, border_color));
        }

        if opaque && self.game_state == GameState::Playing {
            ui.painter()
                .rect(rect, 0.0, opaque_color, Stroke::new(0.5, border_color));
        }
        response
    }
}
