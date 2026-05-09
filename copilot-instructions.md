# Lotto Simulator Project Instructions

## Project Overview
- Project name: `lotto_sim`
- Language: Rust
- Purpose: A lottery simulator and analysis tool for Australian-style lotto games.
- Primary behavior: allow a person to play or simulate playing lottery games until one of two end conditions is reached:
  1. bankruptcy (insufficient funds to continue)
  2. a division 1 win (highest prize)
- Analysis behavior: allow a person to simulate a variable number of games and identify the variance in occurrences per number, including hot/cold number analysis.

## Current Implementation Summary
- The app is built with `eframe` / `egui` and runs as a desktop GUI.
- Lottery configuration is stored in `lotto_config.toml` and deserialized into `AppConfig`.
- The simulator supports multiple game definitions such as `powerball`, `saturday`, and `ozlotto`.
- Users can choose numbers, optionally use auto-pick/hot-number selection, and run both live simulations and fast bulk iterations.
- Simulation state includes:
  - balance
  - total draws
  - total winnings
  - draw history
  - number frequency for hot/cold tracking
  - powerball frequency for PB games

## Rules for Future Agents
1. Respect the existing Rust architecture and maintain idiomatic Rust code.
2. Keep the UI as a desktop app using `eframe`/`egui` unless a clear user request says otherwise.
3. Use `lotto_config.toml` as the source of truth for game rules, prize structures, and cost settings.
4. Preserve the two end conditions:
   - stop when bank balance is insufficient for the next draw cost (bankruptcy)
   - stop when a division 1-level prize is won
5. Preserve support for both live simulation and fast analysis iterations.
6. Ensure hot/cold number variance remains visible and correctly calculated for the selected game pool.
7. Favor clarity and correctness over adding too many new features at once.
8. When modifying the simulator, update or document the configuration expectations.
9. Keep all new code and changes consistent with the existing `Cargo.toml` dependencies and project structure.

## Project Goals
- Deliver a reliable lottery simulation experience.
- Make it easy to compare number frequency outcomes across large sample sizes.
- Make bankruptcy and division 1 win behavior explicit and deterministic.
- Keep the simulation code responsive and safe in the GUI environment.
- Support future enhancements like strategy experiments, ROI metrics, and more advanced reporting.

## Recommended Future Tasks
- Verify and improve the accuracy of prize matching logic across all configured games.
- Add clearer definitions for division 1 wins in the config data and simulation logic.
- Add or improve a results export/save option for simulation runs.
- Add more statistical analysis and graphs for variance, expected value, and ticket performance.

## Usage for Agents
- This file is the authoritative project plan for all future VS Code agents working on `lotto_sim`.
- Use these rules to guide code changes, feature work, bug fixes, and refactoring.
- If a request conflicts with these instructions, prefer the user's explicit request but keep the project goals in mind.
