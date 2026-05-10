use crate::config::{AppConfig, GameConfig};
use rand::prelude::*;
use std::{fs, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};

pub fn generate_draw_numbers(pool_max: u32, draw_count: u32) -> Vec<u32> {
    let mut rng = rand::rng();
    let mut pool: Vec<u32> = (1..=pool_max).collect();
    pool.shuffle(&mut rng);
    pool.truncate(draw_count as usize);
    pool
}

pub fn generate_draw_with_supps(pool_max: u32, draw_count: u32, supps: u32) -> (Vec<u32>, Vec<u32>) {
    let draw = generate_draw_numbers(pool_max, draw_count + supps);
    let main = draw[..draw_count as usize].to_vec();
    let supp = draw[draw_count as usize..].to_vec();
    (main, supp)
}

fn calculate_prize(
    active_cfg: &GameConfig,
    user_nums: &[u32],
    draw_nums: &[u32],
    draw_supps: &[u32],
    draw_pb: Option<u32>,
    is_powerhit: bool,
    user_pb: Option<u32>,
) -> f64 {
    let matches = user_nums.iter().filter(|n| draw_nums.contains(n)).count();
    let supp_matches = user_nums.iter().filter(|n| draw_supps.contains(n)).count();
    let pb_matched = active_cfg.has_powerball && (is_powerhit || user_pb == draw_pb);

    for rule in &active_cfg.prizes {
        if matches == rule.matches
            && supp_matches == rule.supps
            && (!active_cfg.has_powerball || pb_matched == rule.pb)
        {
            return rule.amount;
        }
    }

    0.0
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
    pub game_keys: Vec<String>,
    pub active_game_idx: usize,
    pub selected_numbers: Vec<Vec<u32>>,
    pub selected_powerball: Vec<Option<u32>>,
    pub is_powerhit: Vec<bool>,
    pub current_selected: Vec<u32>,
    pub current_pb: Option<u32>,
    pub current_powerhit: bool,
    pub stats: Arc<Mutex<SimStats>>,
    pub running: Arc<AtomicBool>,
    pub analysis_running: Arc<AtomicBool>,
    pub auto_pick_count: usize,
    pub pre_it_exact: u64,
    pub custom_starting_balance: f64,
}

impl LotteryApp {
    pub fn new(config: AppConfig) -> Self {
        let initial_bal = config.general.starting_balance;
        let mut game_keys: Vec<String> = config.games.keys().cloned().collect();
        game_keys.sort();

        if game_keys.is_empty() {
            panic!("No games defined in lotto_config.toml");
        }

        let stats_data = Self::build_stats(&config);

        Self {
            config,
            game_keys,
            active_game_idx: 0,
            selected_numbers: vec![],
            selected_powerball: vec![],
            is_powerhit: vec![],
            current_selected: vec![],
            current_pb: None,
            current_powerhit: false,
            stats: Arc::new(Mutex::new(stats_data)),
            running: Arc::new(AtomicBool::new(false)),
            analysis_running: Arc::new(AtomicBool::new(false)),
            auto_pick_count: 7,
            pre_it_exact: 1_000_000,
            custom_starting_balance: initial_bal,
        }
    }

    pub fn build_stats(config: &AppConfig) -> SimStats {
        let max_pool = config.games.values().map(|g| g.pool_max).max().unwrap_or(0) as usize;
        let max_pb = config.games.values().filter(|g| g.has_powerball).filter_map(|g| g.powerball_max).max().unwrap_or(0) as usize;

        SimStats {
            balance: config.general.starting_balance,
            total_draws: 0,
            total_won: 0.0,
            history: vec![],
            number_frequency: vec![0; max_pool + 1],
            pb_frequency: vec![0; max_pb + 1],
        }
    }

    pub fn initialize_game_list(&mut self) {
        self.game_keys = self.config.games.keys().cloned().collect();
        self.game_keys.sort();
        if self.active_game_idx >= self.game_keys.len() {
            self.active_game_idx = 0;
        }
    }

    pub fn active_config(&self) -> &GameConfig {
        let key = &self.game_keys[self.active_game_idx];
        &self.config.games[key]
    }

    pub fn active_game_name(&self) -> &str {
        &self.game_keys[self.active_game_idx]
    }

    pub fn reload_config(&mut self) {
        let config: AppConfig = toml::from_str(&fs::read_to_string("lotto_config.toml").unwrap()).unwrap();
        let previous_game = self.active_game_name().to_string();
        self.config = config;
        self.initialize_game_list();
        self.active_game_idx = self.game_keys.iter().position(|name| name == &previous_game).unwrap_or(0);
        self.selected_numbers = vec![];
        self.selected_powerball = vec![];
        self.is_powerhit = vec![];
        self.current_selected.clear();
        self.current_pb = None;
        self.current_powerhit = false;
        self.running.store(false, Ordering::Relaxed);
        self.analysis_running.store(false, Ordering::Relaxed);
        self.stats = Arc::new(Mutex::new(Self::build_stats(&self.config)));
    }

    pub fn select_hot_numbers(&mut self) {
        let s = self.stats.lock().unwrap();
        let active_cfg = self.active_config();
        let pool_limit = active_cfg.pool_max;

        let mut freq_list: Vec<(u32, u64)> = (1..=pool_limit)
            .map(|i| (i as u32, *s.number_frequency.get(i as usize).unwrap_or(&0)))
            .collect();

        if active_cfg.has_powerball {
            let mut pb_list: Vec<(u32, u64)> = (1..=active_cfg.powerball_max.unwrap())
                .map(|i| (i as u32, *s.pb_frequency.get(i as usize).unwrap_or(&0)))
                .collect();
            pb_list.sort_by(|a, b| b.1.cmp(&a.1));
            if let Some(hot_pb) = pb_list.first() {
                if hot_pb.1 > 0 {
                    self.current_pb = Some(hot_pb.0);
                }
            }
        }

        drop(s);

        freq_list.sort_by(|a, b| b.1.cmp(&a.1));
        if self.auto_pick_count == 0 {
            return;
        }

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
        self.current_selected = final_selection;
    }

    pub fn run_fast_iterations(&mut self, iterations: u64) {
        let stats_arc = self.stats.clone();
        let analysis_running = self.analysis_running.clone();
        let active_cfg = self.active_config().clone();
        let pool_max = active_cfg.pool_max;
        let draw_count = active_cfg.draw_count;
        let pb_max = active_cfg.powerball_max.unwrap_or(0);
        let has_powerball = active_cfg.has_powerball;

        analysis_running.store(true, Ordering::Relaxed);
        std::thread::spawn(move || {
            let mut local_freq = vec![0u64; (pool_max + 1) as usize];
            let mut local_pb_freq = vec![0u64; (pb_max + 1) as usize];
            let mut rng = rand::rng();

            for _ in 0..iterations {
                let (winning_nums, winning_supps) = generate_draw_with_supps(pool_max, draw_count, active_cfg.supps);
                for &n in winning_nums.iter().chain(winning_supps.iter()) {
                    local_freq[n as usize] += 1;
                }
                if has_powerball {
                    let pb = rng.random_range(1..=pb_max);
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
            analysis_running.store(false, Ordering::Relaxed);
        });
    }

    pub fn run_simulation(&mut self) {
        let stats_arc = self.stats.clone();
        let running = self.running.clone();
        let active_cfg = self.active_config().clone();
        let user_tickets = self.selected_numbers.clone();
        let user_pbs = self.selected_powerball.clone();
        let powerhits = self.is_powerhit.clone();

        std::thread::spawn(move || {
            let mut rng = rand::rng();
            let total_games: u64 = user_tickets.iter().map(|nums| {
                let base_combs = crate::helpers::combinations(nums.len() as u64, active_cfg.draw_count as u64);
                let game_multiplier = if powerhits[user_tickets.iter().position(|t| t == nums).unwrap()] && active_cfg.has_powerball { 20 } else { 1 };
                base_combs * game_multiplier
            }).sum();
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

                let (winning_nums, winning_supps) = generate_draw_with_supps(active_cfg.pool_max, active_cfg.draw_count, active_cfg.supps);
                for &n in winning_nums.iter().chain(winning_supps.iter()) {
                    if (n as usize) < stats.number_frequency.len() {
                        stats.number_frequency[n as usize] += 1;
                    }
                }

                let draw_pb = if active_cfg.has_powerball { Some(rng.random_range(1..=active_cfg.powerball_max.unwrap())) } else { None };
                if let Some(pb) = draw_pb {
                    stats.pb_frequency[pb as usize] += 1;
                }

                let mut total_prize = 0.0;
                let mut has_div1 = false;
                for (i, user_nums) in user_tickets.iter().enumerate() {
                    let prize = calculate_prize(
                        &active_cfg,
                        user_nums,
                        &winning_nums,
                        &winning_supps,
                        draw_pb,
                        powerhits[i],
                        user_pbs[i],
                    );
                    total_prize += prize;
                    if prize == max_prize {
                        has_div1 = true;
                    }
                }

                stats.balance += total_prize;
                stats.total_won += total_prize;

                if has_div1 {
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
    use super::{calculate_prize, generate_draw_numbers, generate_draw_with_supps};
    use crate::config::{GameConfig, PrizeRule};
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
    fn draw_with_supps_generates_unique_numbers() {
        let pool_max = 45;
        let draw_count = 6;
        let supps = 2;
        let (main, supp) = generate_draw_with_supps(pool_max, draw_count, supps);

        assert_eq!(main.len(), draw_count as usize);
        assert_eq!(supp.len(), supps as usize);
        assert!(main.iter().all(|&n| n >= 1 && n <= pool_max));
        assert!(supp.iter().all(|&n| n >= 1 && n <= pool_max));

        let combined: HashSet<_> = main.iter().chain(supp.iter()).copied().collect();
        assert_eq!(combined.len(), (draw_count + supps) as usize);
    }

    #[test]
    fn calculate_prize_respects_supps() {
        let active_cfg = GameConfig {
            pool_max: 45,
            draw_count: 6,
            supps: 2,
            has_powerball: false,
            cost_per_game: 1.0,
            powerball_max: Some(20),
            prizes: vec![
                PrizeRule { matches: 6, pb: false, supps: 0, amount: 5_000_000.0 },
                PrizeRule { matches: 5, pb: false, supps: 1, amount: 12_000.0 },
                PrizeRule { matches: 5, pb: false, supps: 0, amount: 1_000.0 },
            ],
        };

        let user_nums = vec![1, 2, 3, 4, 5, 6];
        let draw_nums = vec![1, 2, 3, 4, 5, 7];
        let draw_supps = vec![8, 9];

        let prize_without_supp = calculate_prize(&active_cfg, &user_nums, &draw_nums, &draw_supps, None, false, None);
        assert_eq!(prize_without_supp, 1_000.0);

        let draw_supps_match = vec![6, 8];
        let prize_with_supp = calculate_prize(&active_cfg, &user_nums, &draw_nums, &draw_supps_match, None, false, None);
        assert_eq!(prize_with_supp, 12_000.0);
    }

    #[test]
    fn draw_numbers_are_in_range() {
        let pool_max = 49;
        let draw_count = 7;
        let draw = generate_draw_numbers(pool_max, draw_count);

        assert!(draw.iter().all(|&num| num >= 1 && num <= pool_max));
    }
}
