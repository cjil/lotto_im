use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Clone)]
pub struct PrizeRule {
    pub matches: usize,
    #[serde(default)]
    pub pb: bool,
    #[serde(default)]
    pub supps: usize,
    pub amount: f64,
}

#[derive(Deserialize, Clone)]
pub struct GameConfig {
    pub pool_max: u32,
    pub draw_count: u32,
    pub cost_per_game: f64,
    #[serde(default)]
    pub has_powerball: bool,
    #[serde(default)]
    pub supps: u32,
    #[serde(default = "default_powerball_max")]
    pub powerball_max: u32,
    pub prizes: Vec<PrizeRule>,
}

fn default_powerball_max() -> u32 {
    20 // TODO: This needs to go into the lotto_config.toml file
}

#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    #[serde(flatten)]
    pub games: BTreeMap<String, GameConfig>,
}

#[derive(Deserialize, Clone)]
pub struct GeneralConfig {
    pub starting_balance: f64,
}
