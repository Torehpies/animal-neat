use neat::runner;

fn main() {
    // Run a very short XOR evolution for smoke testing
    // Adjust generations/pop_size higher for better results.
    runner::run_neat_xor(true, 1000, 300, 3.9, 2.0);
}
