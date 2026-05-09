# lotto_sim

`lotto_sim` is a Rust lottery simulator and analyzer built with `eframe` / `egui`.

## Project purpose

- Simulate Australian-style lotto games.
- Stop live play on one of two conditions:
  - bankruptcy (insufficient bankroll)
  - Division 1 win (highest prize)
- Support fast statistical analysis of large sample sizes.
- Track and visualize hot/cold number frequency variance.

## Features

- Multiple game modes defined in `lotto_config.toml`
- Configurable bankroll and ticket selection
- Hot number auto-pick based on frequency history
- Live simulation mode with balance and draw tracking
- Fast bulk analysis for number frequency variance
- Graphical charts for variance and balance history

## Running the project

1. Install Rust and Cargo: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
2. Run the app:

   ```powershell
   cargo run
   ```

3. Edit `lotto_config.toml` to adjust game rules, prize amounts, pool sizes, and costs.

## Configuration

The app loads game definitions from `lotto_config.toml` at startup. Each supported game is represented by a top-level table.

### Example configuration

```toml
[powerball]
pool_max = 35
draw_count = 7
has_powerball = true
cost_per_game = 1.575
powerball_max = 20
prizes = [
  { matches = 7, pb = true, amount = 50_000_000.0 },
  { matches = 7, pb = false, amount = 210_000.0 },
  # ...
]
```

### Supported fields

- `pool_max`: highest main ball number
- `draw_count`: number of main balls drawn
- `supps`: number of supplementary balls
- `has_powerball`: whether the game includes a Powerball
- `powerball_max`: Powerball range when enabled
- `cost_per_game`: ticket price
- `prizes`: list of prize rules with `matches`, optional `pb`, optional `supps`, and `amount`

## UI behavior

- The app allows multiple entries per draw, so you can play the same numbers more than once in a single draw.
- Game definitions can be added, removed, or updated in `lotto_config.toml` without recompiling.

## Notes for contributors

- Preserve the GUI desktop app design unless explicitly requested otherwise.
- Keep the two primary stop conditions: bankruptcy and a Division 1 win.
- Keep number frequency tracking and hot/cold variance analysis intact.
- `copilot-instructions.md` documents the project plan and agent guidance.
