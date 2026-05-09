use serde::Deserialize;

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
    pub prizes: Vec<PrizeRule>,
}

#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub powerball: GameConfig,
    pub saturday: GameConfig,
    pub ozlotto: GameConfig,
}

#[derive(Deserialize, Clone)]
pub struct GeneralConfig {
    pub starting_balance: f64,
}
