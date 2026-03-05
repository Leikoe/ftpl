use ftpl::*;

fn main() {
    println!("--- Example 01: Defining Spaces ---");

    // 1. Define a simple 2D space [Height=128, Width=128]
    // Each dimension is a 'Factor' with a role (Kind) and an extent.
    let space = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(128), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(128), Some("W".to_string())),
    ]);

    println!("Space: {:?}", space);
    println!("Volume Extent: {:?}", space.volume_extent());

    // 2. Spaces can be combined using products
    let batch = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(32), Some("B".to_string())),
    ]);
    let batched_space = batch.product(&space);

    println!("Batched Space Factors: {}", batched_space.factors.len()); // 3 factors: B, H, W
}
