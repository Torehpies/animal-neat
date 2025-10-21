# Visualize Ecosystem (example)

This example evolves simple agents with NEAT in a 2D world and renders their behavior using macroquad.

## Run

```bash
cargo run --release --example visualize_ecosystem
```

## Configure

All knobs live in `params.rs`. Notable ones:
- World: `WORLD_W`, `WORLD_H`, `MAX_FOOD`, seasonal knobs
- Movement: inertia (`USE_INERTIA`, `DRAG_COEFF`, `MAX_THRUST`, `MAX_VELOCITY`)
- Fitness: `PLANT_FITNESS`, `MEAT_FITNESS`, `EXPL_WEIGHT`, survival terms
- Communication: thresholds and rewards
- Inputs: enable/disable modalities via `ENABLE_*` flags

## Inputs layout
See comments in `params.rs` and use `sensing::input_ranges()` to avoid hardcoding indices.

## Controls (visual-only)
- P: pause/resume
- F: fast mode
- R: restart episode
- V: show/hide vision cones
- U: show/hide unified input overlay
- E: show/hide energy overlay
- S: save population snapshot

HUD tips
- H: toggle controls section in HUD
- N: toggle best network panel; M: toggle live network panel (focus an agent by clicking)
- K: cycle agent color mode (species/diet)

## Code map
- `sensing.rs` — inputs and perception helpers
- `sim.rs` — physics/movement, predation & digestion, corpse decay
- `world.rs` — food lifecycle and seasonal growth
- `ui/` — world, HUD, and network rendering panels

## Tips
- Start with fewer agents or lower `MAX_FOOD` for performance tests.
- Tune survival/exploration weights small to keep behavior diverse.
- Disable modalities you’re not testing with `ENABLE_*` for cleaner ablations.

## Help Guide slides (in-sim)
The in-simulation Help overlay can show screenshots or diagrams alongside short descriptions.

- Put PNG images in one of these folders (project root relative):
	- `.vscode/assets/guide/` (recommended for repo-local dev assets)
	- `assets/guide/` (alternative path if you prefer a generic assets folder)
- Default filenames referenced by the guide:
	- `overview.png`, `controls.png`, `agents.png`, `graphs.png`
- Missing images are fine — a placeholder frame will be shown.
- To change the slide list or filenames, edit `ui/help_guide.rs` → `preload_help_slides()`.
	Each slide is declared with a title, a description, and an optional image name.
