# lotto_sim

`lotto_sim` is a Rust-based lottery simulator and analyzer built with `eframe` / `egui`.

## Project Purpose
- Simulate playing Australian-style lotto games.
- Run live play until one of two end conditions is reached:
  - bankruptcy (insufficient bankroll to continue)
  - division 1 win (highest prize)
- Support fast statistical analysis of large sample sizes.
- Track and visualize hot/cold number frequency variance.

## Features
- Multiple game modes supported via `lotto_config.toml`
- Configurable bankroll and ticket selection
- Hot number auto-pick based on frequency history
- Live simulation mode with balance tracking
- Bulk fast-iteration analysis for number frequency variance
- Graphical display of number occurrence variance and balance history

## Running the Project
1. Install Rust and Cargo: https://www.rust-lang.org/tools/install
2. Build and run:
   ```powershell
   cargo run
   ```
3. Edit `lotto_config.toml` to adjust game rules, prize amounts, pool sizes, and costs.

## Configuration
- `lotto_config.toml` contains the game configuration used by the simulator.
- The app reads values for each supported game type and prize rules at startup.

## Notes for Contributors
- Keep the GUI desktop app architecture intact unless explicitly requested otherwise.
- Preserve the two main end conditions: bankruptcy and a division 1 win.
- Maintain the number frequency tracking and hot/cold variance analysis.
- `copilot-instructions.md` documents the project plan and agent guidance.
