use ftpl::*;

fn main() {
    println!("--- Example 11: Contiguity and Vectorization Analysis ---");
    let valuation = Valuation::new();

    // 1. Row-Major Layout [4, 8]
    let s4_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l_row_major = Layout::row_major(s4_8.clone());
    
    println!("Row-Major [4, 8]:");
    println!("  Max Vector Width: {}", l_row_major.max_vector_width(&valuation));

    // 2. Transposed Layout (Column-Major)
    let l_transposed = l_row_major.clone().transpose();
    
    println!("\nTransposed (Column-Major) [8, 4]:");
    println!("  Max Vector Width: {}", l_transposed.max_vector_width(&valuation));

    // 3. Tiled Vectorization
    let s_tiled = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(128), Some("Outer".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("Inner".to_string())),
    ]);
    let l_tiled = Layout::row_major(s_tiled);
    
    println!("\nTiled [128, 8] (Logical -> Storage):");
    println!("  Max Vector Width: {}", l_tiled.max_vector_width(&valuation));

    // 4. Broken Contiguity
    let l_complex = l_row_major
        .transpose()
        .reshape(s4_8);

    println!("\nTransposed-Reshaped (Broken Contiguity) [4, 8]:");
    println!("  Max Vector Width: {}", l_complex.max_vector_width(&valuation));
}
