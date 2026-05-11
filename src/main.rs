#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod config;
mod helpers;
mod simulation;
mod ui;

use config::AppConfig;
use std::fs;
use eframe::egui;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let config: AppConfig = toml::from_str(&fs::read_to_string("lotto_config.toml")?)?;

    if args.len() > 1 && args[1] == "batch" {
        return run_batch_mode(&args[2..], &config);
    }

    // GUI mode
    eframe::run_native(
        "Lotto Sim",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 1300.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(simulation::LotteryApp::new(config)))),
    ).map_err(|e| e.into())
}

fn run_batch_mode(args: &[String], config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Lotto Sim - Batch Mode");
    println!("Usage: lotto_sim batch <game> <num_iterations> <ticket_spec1> [<ticket_spec2> ...]");
    println!("  ticket_spec format: NUM1,NUM2,...NUM_N[,PB:N][,POWERHIT]");
    println!("  Example: lotto_sim batch Powerball 1000 1,2,3,4,5,6,7,PB:1 1,2,3,4,5,6,7,POWERHIT\n");

    if args.len() < 3 {
        return Err("Not enough arguments for batch mode".into());
    }

    let game_name = &args[0];
    let iterations: u64 = args[1].parse()?;
    let ticket_specs = parse_ticket_specs(&args[2..])?;

    eprintln!("Running {} iterations of {} with {} ticket(s)...", iterations, game_name, ticket_specs.len());

    let result = simulation::run_batch_simulation(config, game_name, ticket_specs, iterations);
    result.print_summary(game_name);

    Ok(())
}

fn parse_ticket_specs(specs: &[String]) -> Result<Vec<(Vec<u32>, Option<u32>, bool)>, Box<dyn std::error::Error>> {
    let mut tickets = Vec::new();

    for spec in specs {
        let mut numbers = Vec::new();
        let mut powerball = None;
        let mut powerhit = false;

        for part in spec.split(',') {
            if part.starts_with("PB:") {
                powerball = Some(part[3..].parse()?);
            } else if part == "POWERHIT" {
                powerhit = true;
            } else {
                numbers.push(part.parse()?);
            }
        }

        tickets.push((numbers, powerball, powerhit));
    }

    Ok(tickets)
}
