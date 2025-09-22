# Ecosystem Improvement Roadmap

This document lists concrete, incremental upgrades to make the ecosystem richer, more lifelike, and easier to tune and understand. Items are organized in phases so we can implement and validate one by one.

## Phase 1: HUD overhaul and best-network visualization
- [x] Overhaul HUD layout with clear sections and denser, readable stats
- [x] Add a mini network graph of the best-ever genome at the bottom of the HUD
- [x] Update graph live whenever a new best fitness is discovered
- [x] Display speciation info (count) and predation stats per species
- [x] Polish: clamp/truncate overflowing text and support fullscreen start by default

## Phase 2: Corpse decay and digestive lag
- [x] Corpses lose energy over time (decay) to reward timely scavenging
	- Add Agent.corpse_energy and Agent.dead_since
	- On death, initialize corpse_energy = MEAT_ENERGY and start decay each step
	- Scavenging yields remaining corpse_energy, then removes corpse
- [x] Digestive lag for food and meat
	- Add a small queue of DigestEvent per agent; release energy per step
	- Plants: spread FOOD_ENERGY over DIGEST_STEPS_PLANT
	- Meat: spread available energy over DIGEST_STEPS_MEAT
- [x] Headless eval parity with live visualizer
	- Apply identical decay and digestion rules in eval_population_single_episode
- [x] HUD tweaks
	- Display decay rate and digest window sizes
	- Optional: toggle overlay or stats for recent deaths (future)

## Phase 3: Memory and density sensing
- [x] Add memory inputs (last food vector, last danger vector)
- [x] Add low-res density sensors (8 sectors × conspecifics/others)
- [x] Rebalance inputs and network sizes; add input descriptions to HUD
	- Inputs now: per-ray food/wall, current food vector (x,y), energy, last food (x,y), last danger (x,y), density[8]
	- HUD shows danger vector range and density params

## Phase 4: Motor model expansion (sprint/brake)
- [ ] Add sprint and brake outputs; tune energy drain and turn costs while sprinting
- [ ] Small noise added to motor outputs to improve robustness
- [ ] HUD: show current output modes and recent motor usage

## Phase 5: Simple biomes and resource heterogeneity
- [ ] Divide map into a few biomes with distinct plant spawn/spread rates
- [ ] Optional seasonal modulation to nudge migration/exploration
- [ ] HUD: biome indicators and per-biome plant usage stats

## Phase 6: Diversity guardrails
- [ ] Adaptive speciation with a target species band (min/max) instead of a single point
- [ ] Per-species offspring floor (when adjusted fitness > 0) to reduce monocultures
- [ ] HUD: current threshold and target band display

## Phase 7: Fitness shaping and anti-looping
- [ ] Sublinear (sqrt/log) gains for eaten counts to reduce single-strategy domination
- [ ] Diminishing exploration reward for revisits via locality-sensitive hashing
- [ ] Episode randomization (curriculum) to avoid overfitting to one layout

## Phase 8: Obstacles and cover (optional)
- [ ] Add occluding obstacles/cover; adjust vision and pursuit/escape behaviors
- [ ] HUD: toggle to visualize occlusion and cover usage


## General tuning notes
- Introduce new inputs in small batches and watch episode length (energy budget)
- Cap meat energy and add decay/lag to prevent predator runaway
- Keep local density penalties for growth and optionally for fitness stability

---

We’ll start with Phase 1: HUD overhaul + best-network visualization, then proceed sequentially. Each phase includes HUD/telemetry to make tuning tractable.