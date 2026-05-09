#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod config;
mod helpers;
mod simulation;
mod ui;

use config::AppConfig;
use std::fs;

fn main() -> eframe::Result<()> {
    let config: AppConfig = toml::from_str(&fs::read_to_string("lotto_config.toml").unwrap()).unwrap();
    eframe::run_native("Lotto Sim", eframe::NativeOptions::default(), Box::new(|_cc| Ok(Box::new(simulation::LotteryApp::new(config)))))
}
