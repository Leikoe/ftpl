use ftpl::*;

fn main() {
    println!("--- Example 01: High-Level Ergonomic Constructors ---");

    // 1. From Array: row_major([8, 4])
    let l1 = Layout::row_major([8, 4]);
    println!("From Array [8, 4]: Rank {}", l1.source().rank());

    // 2. From Tuple (Mixed Variable & Constant): row_major(("B", 1024))
    // "B" becomes Extent::Variable("B")
    // 1024 becomes Extent::Constant(1024)
    let l2 = Layout::row_major(("B", 1024));
    println!("From Mixed Tuple (\"B\", 1024):");
    println!("  Dim 0 Extent: {:?}", l2.source().factors[0].extent);
    println!("  Dim 1 Extent: {:?}", l2.source().factors[1].extent);

    // 3. Symbolic Evaluation
    let mut valuation = Valuation::new();
    valuation.variables.insert("B".to_string(), 32);
    
    println!("\nSymbolic Evaluation (B=32):");
    println!("  Total Volume: {:?}", valuation.get_extent(&l2.target().volume_extent()));
}
