#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use rand::prelude::*;
use serde::Deserialize;
use std::fs;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};

#[derive(Deserialize, Clone)]
struct PrizeRule {
    matches: usize,
    #[serde(default)]
    pb: bool,
    #[serde(default)]
    supps: usize,
    amount: f64,
}

#[derive(Deserialize, Clone)]
struct GameConfig {
    pool_max: u32,
    draw_count: u32,
    cost_per_game: f64,
    #[serde(default)]
    has_powerball: bool,
    #[serde(default)]
    supps: u32,
    prizes: Vec<PrizeRule>,
}

#[derive(Deserialize, Clone)]
struct AppConfig {
    general: GeneralConfig,
    powerball: GameConfig,
    saturday: GameConfig,
    ozlotto: GameConfig,
}

#[derive(Deserialize, Clone)]
struct GeneralConfig {
    starting_balance: f64,
}

#[derive(PartialEq, Clone, Copy)]
enum LottoType { Saturday, OzLotto, Powerball }

struct SimStats {
    balance: f64,
    total_draws: u64,
    total_won: f64,
    history: Vec<[f64; 2]>,
    number_frequency: Vec<u64>, 
    pb_frequency: Vec<u64>, // Added tracker for Powerball
}

struct LotteryApp {
    config: AppConfig,
    lotto_type: LottoType,
    selected_numbers: Vec<u32>,
    selected_powerball: Option<u32>,
    is_powerhit: bool,
    stats: Arc<Mutex<SimStats>>,
    running: Arc<AtomicBool>,
    auto_pick_count: usize,
    pre_it_exact: u64, 
    custom_starting_balance: f64,
}

impl LotteryApp {
    fn new(config: AppConfig) -> Self {
        let initial_bal = config.general.starting_balance;
        Self {
            config,
            lotto_type: LottoType::Powerball,
            selected_numbers: vec![],
            selected_powerball: None,
            is_powerhit: false,
            stats: Arc::new(Mutex::new(SimStats {
                balance: initial_bal,
                total_draws: 0,
                total_won: 0.0,
                history: vec![],
                number_frequency: vec![0; 50],
                pb_frequency: vec![0; 21], // Tracking 1-20
            })),
            running: Arc::new(AtomicBool::new(false)),
            auto_pick_count: 7,
            pre_it_exact: 1_000_000,
            custom_starting_balance: initial_bal,
        }
    }

    fn select_hot_numbers(&mut self) {
        let s = self.stats.lock().unwrap();
        let pool_limit = match self.lotto_type {
            LottoType::Saturday => self.config.saturday.pool_max,
            LottoType::OzLotto => self.config.ozlotto.pool_max,
            LottoType::Powerball => self.config.powerball.pool_max,
        };

        let mut freq_list: Vec<(u32, u64)> = (1..=pool_limit)
            .map(|i| (i as u32, *s.number_frequency.get(i as usize).unwrap_or(&0)))
            .collect();
        
        // Also auto-select the hottest PB if applicable
        if self.lotto_type == LottoType::Powerball {
            let mut pb_list: Vec<(u32, u64)> = (1..=20)
                .map(|i| (i as u32, *s.pb_frequency.get(i as usize).unwrap_or(&0)))
                .collect();
            pb_list.sort_by(|a, b| b.1.cmp(&a.1));
            if let Some(hot_pb) = pb_list.first() {
                if hot_pb.1 > 0 { self.selected_powerball = Some(hot_pb.0); }
            }
        }
        
        drop(s);

        freq_list.sort_by(|a, b| b.1.cmp(&a.1));
        if self.auto_pick_count == 0 { return; }

        let mut final_selection = Vec::new();
        let mut i = 0;
        while final_selection.len() < self.auto_pick_count && i < freq_list.len() {
            let current_freq = freq_list[i].1;
            let mut tied_group = Vec::new();
            while i < freq_list.len() && freq_list[i].1 == current_freq {
                tied_group.push(freq_list[i].0);
                i += 1;
            }
            let remaining = self.auto_pick_count - final_selection.len();
            if tied_group.len() <= remaining {
                final_selection.extend(tied_group);
            } else {
                let mut rng = rand::rng();
                tied_group.shuffle(&mut rng);
                final_selection.extend(&tied_group[..remaining]);
            }
        }
        final_selection.sort();
        self.selected_numbers = final_selection;
    }

    fn run_fast_iterations(&mut self, iterations: u64) {
        let stats_arc = self.stats.clone();
        let lotto_type = self.lotto_type;
        let pool_max = match lotto_type {
            LottoType::Saturday => self.config.saturday.pool_max,
            LottoType::OzLotto => self.config.ozlotto.pool_max,
            LottoType::Powerball => self.config.powerball.pool_max,
        };
        let draw_count = match lotto_type {
            LottoType::Saturday => self.config.saturday.draw_count,
            LottoType::OzLotto => self.config.ozlotto.draw_count,
            LottoType::Powerball => self.config.powerball.draw_count,
        };

        std::thread::spawn(move || {
            let mut local_freq = vec![0u64; (pool_max + 1) as usize];
            let mut local_pb_freq = vec![0u64; 21];
            let mut rng = rand::rng();
            let mut pool: Vec<u32> = (1..=pool_max).collect();

            for _ in 0..iterations {
                pool.shuffle(&mut rng);
                for i in 0..draw_count as usize {
                    let n = pool[i];
                    local_freq[n as usize] += 1;
                }
                if lotto_type == LottoType::Powerball {
                    let pb = rng.random_range(1..=20);
                    local_pb_freq[pb as usize] += 1;
                }
            }

            let mut stats = stats_arc.lock().unwrap();
            if stats.number_frequency.len() < local_freq.len() {
                stats.number_frequency.resize(local_freq.len(), 0);
            }
            for (i, val) in local_freq.iter().enumerate() {
                stats.number_frequency[i] += val;
            }
            for (i, val) in local_pb_freq.iter().enumerate() {
                stats.pb_frequency[i] += val;
            }
        });
    }
}

fn combinations(n: u64, k: u64) -> u64 {
    if k > n { return 0; }
    if k == 0 || k == n { return 1; }
    let k = k.min(n - k);
    let mut res = 1;
    for i in 1..=k { res = res * (n - i + 1) / i; }
    res
}

fn format_with_separators(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { result.push(','); }
        result.push(c);
    }
    result
}

fn format_currency(n: f64) -> String {
    let is_neg = n < 0.0;
    let s = format!("{:.0}", n.abs());
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { result.push(','); }
        result.push(c);
    }
    if is_neg { format!("-${}", result) } else { format!("${}", result) }
}

impl eframe::App for LotteryApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}

impl LotteryApp {
    fn render_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.heading("Aussie Lotto Sim - High Speed Analyser");

            ui.horizontal(|ui| {
                if ui.selectable_value(&mut self.lotto_type, LottoType::Saturday, "Saturday").clicked() { self.selected_numbers.clear(); }
                if ui.selectable_value(&mut self.lotto_type, LottoType::OzLotto, "Oz Lotto").clicked() { self.selected_numbers.clear(); }
                if ui.selectable_value(&mut self.lotto_type, LottoType::Powerball, "Powerball").clicked() { self.selected_numbers.clear(); }
            });

            let (pool_max, draw_count, cost_per_game, has_powerball) = {
                let cfg = match self.lotto_type {
                    LottoType::Saturday => &self.config.saturday,
                    LottoType::OzLotto => &self.config.ozlotto,
                    LottoType::Powerball => &self.config.powerball,
                };
                (cfg.pool_max, cfg.draw_count, cfg.cost_per_game, cfg.has_powerball)
            };

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Analysis & Setup").strong().color(egui::Color32::LIGHT_BLUE));
                
                
                ui.horizontal(|ui| {
				    ui.label("Bankroll:");
				    let mut cash_str = format_currency(self.custom_starting_balance);
				    if ui.add(egui::TextEdit::singleline(&mut cash_str).desired_width(200.0)).changed() {
				        let clean_str = cash_str.replace(',', "").replace('$', "");
				        if let Ok(val) = clean_str.parse::<f64>() {
				            self.custom_starting_balance = val.clamp(0.0, 1_000_000_000.0);
				        }
				    }
                    if ui.button("Update Bank").clicked() {
                        let mut s = self.stats.lock().unwrap();
                        s.balance = self.custom_starting_balance;
                    }
				});

    
                ui.horizontal(|ui| {
				    ui.label("Sample Size:");
				    let mut iterations_str = format_with_separators(self.pre_it_exact as u64);
				    if ui.add(egui::TextEdit::singleline(&mut iterations_str).desired_width(200.0)).changed() {
				        // Remove commas before parsing back to a number
				        let clean_str = iterations_str.replace(',', "");
				        if let Ok(val) = clean_str.parse::<u64>() {
				            self.pre_it_exact = val.clamp(1, 1_000_000_000);
				        }
				    }
                    if ui.button("Run analysis").clicked() {
                        self.run_fast_iterations(self.pre_it_exact);
                    }
				});


                
                ui.separator();
                
                ui.horizontal(|ui| {
                    ui.label("Auto-pick Top:");
                    ui.add(egui::DragValue::new(&mut self.auto_pick_count).range(1..=20));
                    if ui.button("Select High Frequency").clicked() {
                        self.select_hot_numbers();
                    }
                });

                if has_powerball {
                    ui.checkbox(&mut self.is_powerhit, "Powerhit (Guarantees Powerball)");
                }

                let base_games = combinations(self.selected_numbers.len() as u64, draw_count as u64);
                let game_multiplier = if self.is_powerhit && has_powerball { 20 } else { 1 };
                let total_games = base_games * game_multiplier;
                
                ui.colored_label(egui::Color32::GOLD, format!("Games: {} | Draw Cost: {}", format_with_separators(total_games), format_currency(total_games as f64 * cost_per_game)));

                egui::Grid::new("num_grid").spacing([5.0, 5.0]).show(ui, |ui| {
                    for i in 1..=pool_max {
                        let is_selected = self.selected_numbers.contains(&i);
                        if ui.selectable_label(is_selected, i.to_string()).clicked() {
                            if is_selected { self.selected_numbers.retain(|&x| x != i); }
                            else if self.selected_numbers.len() < 20 { self.selected_numbers.push(i); self.selected_numbers.sort(); }
                        }
                        if i % 10 == 0 { ui.end_row(); }
                    }
                });

                if has_powerball && !self.is_powerhit {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("PB:");
                        for i in 1..=20 {
                            if ui.selectable_label(self.selected_powerball == Some(i), i.to_string()).clicked() {
                                self.selected_powerball = Some(i);
                            }
                        }
                    });
                }
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Hot/Cold Variance (Main Pool)");
                let s = self.stats.lock().unwrap();
                
                let min_freq = (1..=pool_max)
                    .map(|i| *s.number_frequency.get(i as usize).unwrap_or(&0))
                    .min()
                    .unwrap_or(0);

                let bars: Vec<Bar> = (1..=pool_max)
                    .map(|i| {
                        let freq = *s.number_frequency.get(i as usize).unwrap_or(&0);
                        let variance = freq.saturating_sub(min_freq);
                        Bar::new(i as f64, variance as f64)
                            .name(format!("Num {} (+{})", i, format_with_separators(variance)))
                            .fill(egui::Color32::from_rgb(100, 150, 250))
                    })
                    .collect();
                
                Plot::new("freq_chart")
                    .view_aspect(3.0).height(120.0).include_y(0.0)
                    .show(ui, |plot_ui| { plot_ui.bar_chart(BarChart::new("Variance", bars).width(0.6)); });

                if has_powerball {
                    ui.separator();
                    ui.label("Powerball Variance");
                    let pb_min = (1..=20).map(|i| *s.pb_frequency.get(i as usize).unwrap_or(&0)).min().unwrap_or(0);
                    let pb_bars: Vec<Bar> = (1..=20)
                        .map(|i| {
                            let freq = *s.pb_frequency.get(i as usize).unwrap_or(&0);
                            let var = freq.saturating_sub(pb_min);
                            Bar::new(i as f64, var as f64).name(format!("PB {} (+{})", i, var)).fill(egui::Color32::from_rgb(250, 100, 100))
                        })
                        .collect();
                    Plot::new("pb_chart")
                        .view_aspect(3.0).height(100.0).include_y(0.0)
                        .show(ui, |plot_ui| { plot_ui.bar_chart(BarChart::new("PB Variance", pb_bars).width(0.6)); });
                }
            });

            ui.add_space(10.0);
            {
                let s = self.stats.lock().unwrap();
                ui.columns(3, |cols| {
                    cols[0].label("Bank Balance"); cols[0].heading(format_currency(s.balance));
                    cols[1].label("Total Winnings"); cols[1].heading(egui::RichText::new(format_currency(s.total_won)).color(egui::Color32::GREEN));
                    cols[2].label("Total Draws Played"); cols[2].heading(format_with_separators(s.total_draws));
                });
            }

            ui.horizontal(|ui| {
                let is_running = self.running.load(Ordering::Relaxed);
                let can_start = self.selected_numbers.len() >= draw_count as usize && (!has_powerball || self.is_powerhit || self.selected_powerball.is_some());
                
                if ui.add_enabled(can_start, egui::Button::new(if is_running { "STOP LIVE SIM" } else { "START LIVE SIM" })).clicked() {
                    let next = !is_running;
                    self.running.store(next, Ordering::Relaxed);
                    if next { self.run_simulation(); }
                }
                if ui.button("RESET ALL").clicked() {
                    self.running.store(false, Ordering::Relaxed);
                    let mut s = self.stats.lock().unwrap();
                    *s = SimStats { 
                        balance: self.custom_starting_balance, 
                        total_draws: 0, 
                        total_won: 0.0, 
                        history: vec![], 
                        number_frequency: vec![0; (pool_max + 1) as usize],
                        pb_frequency: vec![0; 21],
                    };
                }
            });

            let points: PlotPoints = {
                let s = self.stats.lock().unwrap();
                s.history.iter().map(|&[x, y]| [x, y]).collect()
            };
            
            Plot::new("history").view_aspect(4.0).height(150.0).y_axis_formatter(|mark, _| format_currency(mark.value)).show(ui, |plot_ui| { 
                plot_ui.line(Line::new("Balance", points).color(egui::Color32::GREEN)); 
            });

            if self.running.load(Ordering::Relaxed) { ui.ctx().request_repaint(); }
        });
    }

    fn run_simulation(&mut self) {
        let stats_arc = self.stats.clone();
        let running = self.running.clone();
        let lotto_type = self.lotto_type;
        let config = self.config.clone();
        let user_nums = self.selected_numbers.clone();
        let user_pb = self.selected_powerball;
        let is_powerhit = self.is_powerhit;

        std::thread::spawn(move || {
            let mut rng = rand::rng();
            let active_cfg = match lotto_type {
                LottoType::Saturday => &config.saturday,
                LottoType::OzLotto => &config.ozlotto,
                LottoType::Powerball => &config.powerball,
            };

            let base_combs = combinations(user_nums.len() as u64, active_cfg.draw_count as u64);
            let total_games = base_combs * if is_powerhit && active_cfg.has_powerball { 20 } else { 1 };
            let cost_per_draw = total_games as f64 * active_cfg.cost_per_game;
            let max_prize = active_cfg.prizes.iter().map(|p| p.amount).fold(0.0, f64::max);

            while running.load(Ordering::Relaxed) {
                let mut stats = stats_arc.lock().unwrap();

                if stats.balance < cost_per_draw {
                    running.store(false, Ordering::Relaxed);
                    drop(stats);
                    break;
                }

                stats.total_draws += 1;
                stats.balance -= cost_per_draw;

                let mut pool: Vec<u32> = (1..=active_cfg.pool_max).collect();
                pool.shuffle(&mut rng);
                let winning_nums = &pool[0..active_cfg.draw_count as usize];
                
                for &n in winning_nums {
                    if (n as usize) < stats.number_frequency.len() {
                        stats.number_frequency[n as usize] += 1;
                    }
                }

                let draw_pb = if active_cfg.has_powerball { rng.random_range(1..=20) } else { 0 };
                if active_cfg.has_powerball { stats.pb_frequency[draw_pb as usize] += 1; }

                let pb_matched = is_powerhit || (active_cfg.has_powerball && user_pb == Some(draw_pb));
                let matches = user_nums.iter().filter(|n| winning_nums.contains(n)).count();

                let mut prize = 0.0;
                for rule in &active_cfg.prizes {
                    if matches == rule.matches && (!active_cfg.has_powerball || pb_matched == rule.pb) {
                        prize = rule.amount; 
                        break;
                    }
                }

                stats.balance += prize;
                stats.total_won += prize;

                if prize >= max_prize && prize > 0.0 {
                    running.store(false, Ordering::Relaxed);
                    let d = stats.total_draws as f64;
                    let b = stats.balance;
                    stats.history.push([d, b]);
                    drop(stats);
                    break;
                }

                if stats.total_draws % 1000 == 0 {
                    let d = stats.total_draws as f64;
                    let b = stats.balance;
                    stats.history.push([d, b]);
                }
                drop(stats);
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let config: AppConfig = toml::from_str(&fs::read_to_string("lotto_config.toml").unwrap()).unwrap();
    eframe::run_native("Lotto Sim", eframe::NativeOptions::default(), Box::new(|_cc| Ok(Box::new(LotteryApp::new(config)))))
}
