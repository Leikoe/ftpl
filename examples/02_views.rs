use ftpl::*;

fn main() {
    println!("--- Example 02: Layout Views (Transpose & Reshape) ---");

    // Start with a [Height=4, Width=8] tensor
    let s = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);

    // 1. Transpose: (H, W) -> (W, H)
    let transpose = Expression::Permute(s.clone(), vec![1, 0]);
    println!(
        "Transpose Target Space: {:?}",
        transpose.target().volume_extent()
    );

    // 2. Reshape: Flatten (H, W) into (H*W)
    let flattened_space = Space::new(vec![Factor::new(
        Kind::Logical,
        Extent::Constant(32),
        Some("HW".to_string()),
    )]);
    let reshape = Expression::Reshape(s.clone(), flattened_space.clone());
    println!(
        "Reshape Target Space: {:?}",
        reshape.target().volume_extent()
    );

    // 3. Coordinate Mapping
    let valuation = Valuation::new();
    let input_coord = vec![1, 2]; // (h=1, w=2)
    let output_coord = reshape.apply(&valuation, &input_coord).unwrap();

    // In row-major (4, 8), (1, 2) maps to offset 1*8 + 2 = 10
    println!(
        "Coordinate (1, 2) in [4, 8] maps to {:?} in [32]",
        output_coord
    );
}
