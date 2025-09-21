fn main() {
    // Use the library API (crate root re-exports `neat::*`).
    neat::runner::run_neat_xor(false, 1000, 50, 3.9, 2.0);
}
