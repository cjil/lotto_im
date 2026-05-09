use crate::helpers::{format_currency, format_with_separators};
use crate::simulation::LotteryApp;
use eframe::egui;
use eframe::egui::ScrollArea;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use std::sync::atomic::Ordering;

impl eframe::App for LotteryApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}

impl LotteryApp {
    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        let active_cfg = self.active_config();
        let pool_max = active_cfg.pool_max;
        let draw_count = active_cfg.draw_count;
        let cost_per_game = active_cfg.cost_per_game;
        let has_powerball = active_cfg.has_powerball;
        let pb_max = active_cfg.powerball_max;

        ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.heading("Aussie Lotto Sim - High Speed Analyser");

            ui.horizontal(|ui| {
                for (game_idx, game_name) in self.game_keys.iter().enumerate() {
                    if ui.selectable_label(self.active_game_idx == game_idx, game_name).clicked() {
                        self.active_game_idx = game_idx;
                        self.selected_numbers.clear();
                        self.selected_powerball = None;
                        self.is_powerhit = false;
                    }
                }
                if ui.button("Reload Config").clicked() {
                    self.reload_config();
                }
            });

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
                    self.auto_pick_count = self.auto_pick_count.clamp(1, pool_max as usize);
                    ui.label("Auto-pick Top:");
                    ui.add(egui::DragValue::new(&mut self.auto_pick_count).range(1..=pool_max as usize));
                    if ui.button("Select High Frequency").clicked() {
                        self.select_hot_numbers();
                    }
                });

                if has_powerball {
                    ui.checkbox(&mut self.is_powerhit, "Powerhit (Guarantees Powerball)");
                }

                ui.horizontal(|ui| {
                    ui.label("Entries per draw:");
                    ui.add(egui::DragValue::new(&mut self.ticket_multiplier).range(1..=50));
                });

                let base_games = crate::helpers::combinations(self.selected_numbers.len() as u64, draw_count as u64);
                let ticket_multiplier = self.ticket_multiplier as u64;
                let game_multiplier = if self.is_powerhit && has_powerball { 20 } else { 1 };
                let total_games = base_games * ticket_multiplier * game_multiplier;

                ui.colored_label(
                    egui::Color32::GOLD,
                    format!(
                        "Entries: {} | Games: {} | Draw Cost: {}",
                        format_with_separators(ticket_multiplier),
                        format_with_separators(total_games),
                        format_currency(total_games as f64 * cost_per_game)
                    ),
                );

                egui::Grid::new("num_grid").spacing([5.0, 5.0]).show(ui, |ui| {
                    for i in 1..=pool_max {
                        let is_selected = self.selected_numbers.contains(&i);
                        if ui.selectable_label(is_selected, i.to_string()).clicked() {
                            if is_selected {
                                self.selected_numbers.retain(|&x| x != i);
                            } else if self.selected_numbers.len() < 20 {
                                self.selected_numbers.push(i);
                                self.selected_numbers.sort();
                            }
                        }
                        if i % 10 == 0 {
                            ui.end_row();
                        }
                    }
                });

                if has_powerball && !self.is_powerhit {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("PB:");
                        for i in 1..=pb_max {
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

                let mut max_variance: f64 = 1.0;
                let bars: Vec<Bar> = (1..=pool_max)
                    .map(|i| {
                        let freq = *s.number_frequency.get(i as usize).unwrap_or(&0);
                        let variance = freq.saturating_sub(min_freq) as f64;
                        max_variance = max_variance.max(variance);
                        Bar::new(i as f64, variance)
                            .name(format!("Num {} (+{})", i, format_with_separators(freq.saturating_sub(min_freq))))
                            .fill(egui::Color32::from_rgb(100, 150, 250))
                    })
                    .collect();

                Plot::new("freq_chart")
                    .height(180.0)
                    .allow_scroll([true, false])
                    .allow_drag([true, false])
                    .allow_zoom([true, false])
                    .allow_boxed_zoom(false)
                    .default_y_bounds(0.0, max_variance * 1.05)
                    .show(ui, |plot_ui| {
                        plot_ui.bar_chart(BarChart::new("Variance", bars).width(0.6));
                    });

                if has_powerball {
                    ui.separator();
                    ui.label("Powerball Variance");
                    let pb_min = (1..=pb_max)
                        .map(|i| *s.pb_frequency.get(i as usize).unwrap_or(&0))
                        .min()
                        .unwrap_or(0);
                    let mut max_pb_variance: f64 = 1.0;
                    let pb_bars: Vec<Bar> = (1..=pb_max)
                        .map(|i| {
                            let freq = *s.pb_frequency.get(i as usize).unwrap_or(&0);
                            let var = freq.saturating_sub(pb_min) as f64;
                            max_pb_variance = max_pb_variance.max(var);
                            Bar::new(i as f64, var)
                                .name(format!("PB {} (+{})", i, freq.saturating_sub(pb_min)))
                                .fill(egui::Color32::from_rgb(250, 100, 100))
                        })
                        .collect();
                    Plot::new("pb_chart")
                        .height(150.0)
                        .allow_scroll([true, false])
                        .allow_drag([true, false])
                        .allow_zoom([true, false])
                        .allow_boxed_zoom(false)
                        .default_y_bounds(0.0, max_pb_variance * 1.05)
                        .show(ui, |plot_ui| {
                            plot_ui.bar_chart(BarChart::new("PB Variance", pb_bars).width(0.6));
                        });
                }
            });

            ui.add_space(10.0);
            {
                let s = self.stats.lock().unwrap();
                ui.columns(3, |cols| {
                    cols[0].label("Bank Balance");
                    cols[0].heading(format_currency(s.balance));
                    cols[1].label("Total Winnings");
                    cols[1].heading(egui::RichText::new(format_currency(s.total_won)).color(egui::Color32::GREEN));
                    cols[2].label("Total Draws Played");
                    cols[2].heading(format_with_separators(s.total_draws));
                });
            }

            ui.horizontal(|ui| {
                let is_running = self.running.load(Ordering::Relaxed);
                let can_start = self.selected_numbers.len() >= draw_count as usize
                    && (!has_powerball || self.is_powerhit || self.selected_powerball.is_some());

                if ui
                    .add_enabled(
                        can_start,
                        egui::Button::new(if is_running { "STOP LIVE SIM" } else { "START LIVE SIM" }),
                    )
                    .clicked()
                {
                    let next = !is_running;
                    self.running.store(next, Ordering::Relaxed);
                    if next {
                        self.run_simulation();
                    }
                }
                if ui.button("RESET ALL").clicked() {
                    self.running.store(false, Ordering::Relaxed);
                    let mut s = self.stats.lock().unwrap();
                    *s = crate::simulation::SimStats {
                        balance: self.custom_starting_balance,
                        total_draws: 0,
                        total_won: 0.0,
                        history: vec![],
                        number_frequency: vec![0; (pool_max + 1) as usize],
                        pb_frequency: vec![0; (pb_max + 1) as usize],
                    };
                }
            });

            let points: PlotPoints = {
                let s = self.stats.lock().unwrap();
                s.history.iter().map(|&[x, y]| [x, y]).collect()
            };
            let max_balance = {
                let s = self.stats.lock().unwrap();
                s.history.iter().map(|&[_x, y]| y).fold(s.balance, f64::max)
            };

            Plot::new("history")
                .height(225.0)
                .allow_scroll([true, false])
                .allow_drag([true, false])
                .allow_zoom([true, false])
                .allow_boxed_zoom(false)
                .default_y_bounds(0.0, (max_balance * 1.05).max(1.0))
                .y_axis_formatter(|mark, _| format_currency(mark.value))
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new("Balance", points).color(egui::Color32::GREEN));
                });

            if self.running.load(Ordering::Relaxed) {
                ui.ctx().request_repaint();
            }
        });
    }
}
