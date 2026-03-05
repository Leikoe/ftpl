use ftpl::*;

fn main() {
    println!("--- Example 11: Contiguity and Vectorization Analysis ---");
    let valuation = Valuation::new();

    // 1. Row-Major Layout [4, 8]
    // The innermost dimension (Width=8) is contiguous.
    let s4_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l_row_major = Expression::Linearize(s4_8.clone());
    
    println!("Row-Major [4, 8]:");
    println!("  Max Vector Width: {}", l_row_major.max_vector_width(&valuation));
    // Expected: 8 (the width of the innermost dimension)

    // 2. Transposed Layout (Column-Major)
    // We transpose [4, 8] to [8, 4] but access it such that H is now innermost.
    // However, in memory it's still stored row-major.
    let transpose = Expression::Permute(s4_8.clone(), vec![1, 0]);
    let l_transposed = Expression::Composition(Box::new(transpose), Box::new(l_row_major.clone()));
    
    println!("\nTransposed (Column-Major) [8, 4]:");
    println!("  Max Vector Width: {}", l_transposed.max_vector_width(&valuation));
    // Expected: 1 (Stride is now 8, not 1)

    // 3. Tiled Vectorization
    // A kernel iterates over a [128, 8] logical space.
    // We want to know if the inner '8' dimension is contiguous in memory.
    let s_tiled = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(128), Some("Outer".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("Inner".to_string())),
    ]);
    let l_tiled = Expression::Linearize(s_tiled.clone());
    
    println!("\nTiled [128, 8] (Logical -> Storage):");
    println!("  Max Vector Width: {}", l_tiled.max_vector_width(&valuation));
    // Expected: 8

    // 4. Broken Contiguity (from Example 06)
    // row_major(4, 8).T.reshape(4, 8)
    let s8_4 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
    ]);
    let t = Expression::Permute(s8_4.clone(), vec![1, 0]);
    let r = Expression::Reshape(s4_8.clone(), s8_4.clone());
    let l_complex = Expression::Composition(Box::new(r), Box::new(Expression::Composition(Box::new(t), Box::new(l_row_major))));

    println!("\nTransposed-Reshaped (Broken Contiguity) [4, 8]:");
    println!("  Max Vector Width: {}", l_complex.max_vector_width(&valuation));
    // Expected: 1 (The indices are scrambled)

    println!("\nSUMMARY:");
    println!("  The compiler uses this 'max_vector_width' to decide if it can");
    println!("  emit float4, float8, or bit-mapped vector instructions.");
}
