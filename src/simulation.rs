use crate::config::AppConfig;
use rand::prelude::*;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};

#[derive(PartialEq, Clone, Copy)]
pub enum LottoType { Saturday, OzLotto, Powerball }

pub fn generate_draw_numbers(pool_max: u32, draw_count: u32) -> Vec<u32> {
    let mut rng = rand::rng();
    let mut pool: Vec<u32> = (1..=pool_max).collect();
    pool.shuffle(&mut rng);
    pool.truncate(draw_count as usize);
    pool
}

pub struct SimStats {
    pub balance: f64,
    pub total_draws: u64,
    pub total_won: f64,
    pub history: Vec<[f64; 2]>,
    pub number_frequency: Vec<u64>,
    pub pb_frequency: Vec<u64>,
}

pub struct LotteryApp {
    pub config: AppConfig,
    pub lotto_type: LottoType,
    pub selected_numbers: Vec<u32>,
    pub selected_powerball: Option<u32>,
    pub is_powerhit: bool,
    pub stats: Arc<Mutex<SimStats>>,
    pub running: Arc<AtomicBool>,
    pub auto_pick_count: usize,
    pub pre_it_exact: u64,
    pub custom_starting_balance: f64,
}

impl LotteryApp {
    pub fn new(config: AppConfig) -> Self {
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
                pb_frequency: vec![0; 21],
            })),
            running: Arc::new(AtomicBool::new(false)),
            auto_pick_count: 7,
            pre_it_exact: 1_000_000,
            custom_starting_balance: initial_bal,
        }
    }

    pub fn select_hot_numbers(&mut self) {
        let s = self.stats.lock().unwrap();
        let pool_limit = match self.lotto_type {
            LottoType::Saturday => self.config.saturday.pool_max,
            LottoType::OzLotto => self.config.ozlotto.pool_max,
            LottoType::Powerball => self.config.powerball.pool_max,
        };

        let mut freq_list: Vec<(u32, u64)> = (1..=pool_limit)
            .map(|i| (i as u32, *s.number_frequency.get(i as usize).unwrap_or(&0)))
            .collect();

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

    pub fn run_fast_iterations(&mut self, iterations: u64) {
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

            for _ in 0..iterations {
                let winning_nums = generate_draw_numbers(pool_max, draw_count);
                for &n in &winning_nums {
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

    pub fn run_simulation(&mut self) {
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

            let base_combs = crate::helpers::combinations(user_nums.len() as u64, active_cfg.draw_count as u64);
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

let winning_nums = generate_draw_numbers(active_cfg.pool_max, active_cfg.draw_count);

                for &n in &winning_nums {
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

#[cfg(test)]
mod tests {
    use super::generate_draw_numbers;
    use std::collections::HashSet;

    #[test]
    fn draw_generates_expected_length() {
        let pool_max = 45;
        let draw_count = 6;
        let draw = generate_draw_numbers(pool_max, draw_count);

        assert_eq!(draw.len(), draw_count as usize);
    }

    #[test]
    fn draw_numbers_are_unique() {
        let pool_max = 45;
        let draw_count = 6;
        let draw = generate_draw_numbers(pool_max, draw_count);

        let unique: HashSet<_> = draw.iter().copied().collect();
        assert_eq!(unique.len(), draw_count as usize);
    }

    #[test]
    fn draw_numbers_are_in_range() {
        let pool_max = 49;
        let draw_count = 7;
        let draw = generate_draw_numbers(pool_max, draw_count);

        assert!(draw.iter().all(|&num| num >= 1 && num <= pool_max));
    }
}
