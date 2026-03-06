use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // 1. Define a 2D logical tensor [8, 8]
    let logical = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);

    // 2. Define a Hardware Target: [4 Threads, 16 local storage offsets]
    // A mapping that tiles the 8x8 into 2x2 blocks per thread.
    let target = Space::new(vec![
        Factor::new(
            Kind::Execution,
            Extent::Constant(4),
            Some("ThreadID".to_string()),
        ),
        Factor::new(
            Kind::Storage,
            Extent::Constant(16),
            Some("LocalOffset".to_string()),
        ),
    ]);

    // 3. Create the mapping: Logical -> (ThreadID, Offset)
    // We'll use a simple reshape to distribute work.
    let mapping = Expression::Reshape(logical, target);

    // 4. Render it
    let svg = viz::render_svg(&mapping, &valuation);

    let mut file = File::create("hardware_mapping.svg").unwrap();
    file.write_all(svg.as_bytes()).unwrap();

    println!("Saved hardware_mapping.svg");
    println!("Each color represents a different Execution Unit (Thread).");
    println!(
        "The text 'T{{id}}:{{off}}' shows which thread owns the cell and its local memory offset."
    );
}
