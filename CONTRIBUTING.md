# Contributing

Thanks for your interest in improving this project! A few guidelines to keep things smooth:

## Project layout
- Core NEAT implementation lives under `src/neat/*`.
- The interactive demo lives in `examples/visualize_ecosystem/*`.
- All simulation knobs go in `examples/visualize_ecosystem/params.rs`.

## Coding conventions
- Prefer explicit constants in `params.rs` over magic numbers in code.
- Document new parameters with a short rationale comment.
- Keep rendering concerns in `examples/visualize_ecosystem/ui/*`.
- Use `sensing::input_ranges()` for any input indexing logic.
- Avoid breaking the input/output layout; if you must, provide a migration note.

## Testing and validation
- `cargo build` should be clean. For smoother visuals, `cargo run --release --example visualize_ecosystem`.
- When changing public behavior, add a sentence to the README “Developing” section or a short changelog note.
- For performance-sensitive changes, spot-check frame rate in release mode.

## Snapshots and reproducibility
- The visualizer supports saving population snapshots. If you add new metadata, include parameter hashes or summaries to aid reproducibility.

## Pull requests
- Keep PRs focused. Include:
  - What changed and why
  - Any notable parameter changes
  - Validation steps (how you tested)

Thanks again for contributing!
