use ftpl::*;
use std::collections::HashMap;

fn main() {
    println!("=== FTPL Walkthrough: From DL Ops to Hardware Fit ===\n");

    // --- STEP 1: Define the initial Logical Tensor ---
    // A 4D tensor common in CV: [Batch=32, Channels=64, Height=128, Width=128]
    let logical_space = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(32), Some("B".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(64), Some("C".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(128), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(128), Some("W".to_string())),
    ]);
    println!("[1] Defined Logical Space: {:?}", logical_space.volume_extent());

    // --- STEP 2: Apply a Deep Learning View (Transpose to Channels-Last) ---
    // (B, C, H, W) -> (B, H, W, C)
    let transpose = Expression::Permute(logical_space.clone(), vec![0, 3, 1, 2]);
    println!("[2] Applied Transpose (NHWC). Target Space: {:?}", transpose.target().volume_extent());

    // --- STEP 3: Apply a Reshape (Flatten Spatial Dimensions) ---
    // (B, H, W, C) -> (B, H*W, C)
    let flattened_space = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(32), Some("B".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(128 * 128), Some("HW".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(64), Some("C".to_string())),
    ]);
    // A Reshape is a bijective mapping via a linearized offset
    let reshape = Expression::Reshape(transpose.target(), flattened_space.clone());
    
    // Compose them: Final = Reshape o Transpose
    let full_view = Expression::Composition(Box::new(transpose), Box::new(reshape));
    println!("[3] Applied Reshape. Final View Source: {:?} -> Target: {:?}", 
             full_view.source().volume_extent(), full_view.target().volume_extent());

    // --- STEP 4: Compiler Analysis (Tiling & Vectorization) ---
    // Split the 'C' dimension (idx 2 in flattened space) into [Tile=8, Vector=8]
    // We'll just demonstrate the principle on a 1D slice of 64 channels.
    let c_space = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(64), None)]);
    let tiled_c = Space::new(vec![
        Factor::new(Kind::Tile, Extent::Constant(8), Some("Outer".to_string())),
        Factor::new(Kind::Instruction, Extent::Constant(8), Some("Inner".to_string())),
    ]);
    let tiling_expr = Expression::Reshape(c_space, tiled_c);
    
    // Symbolic Lowering: Show the device-side address calculation
    let valuation = Valuation::new();
    let lowered = tiling_expr.lower(&valuation, vec![ScalarExpr::Input(0)]);
    println!("[4] Compiler Tiling Analysis:");
    println!("    Lowered Address Expr: {:?}", lowered.0[0]);
    // Note: The coefficient of the inner dimension is 1, indicating contiguity.

    // --- STEP 5: Hardware Primitive Fit (Tensor Core MMA) ---
    // Suppose we have a 16x16 Tensor Core fragment we want to fit onto our HW dimension.
    let fragment_16x16 = Space::new(vec![
        Factor::new(Kind::Fragment, Extent::Constant(16), None),
        Factor::new(Kind::Fragment, Extent::Constant(16), None),
    ]);
    let tensor_core_primitive = Expression::Linearize(fragment_16x16);

    // Our program uses a 32x32 tiled layout: (Fragment x Tile)
    let tile_2x2 = Space::new(vec![
        Factor::new(Kind::Tile, Extent::Constant(2), None),
        Factor::new(Kind::Tile, Extent::Constant(2), None),
    ]);
    let program_layout = Expression::Product(
        Box::new(tensor_core_primitive.clone()),
        Box::new(Expression::Identity(tile_2x2))
    );

    println!("[5] Hardware Primitive Fit Checking:");
    if let Some(remainder) = program_layout.left_div(tensor_core_primitive) {
        println!("    SUCCESS: Hardware Primitive fits!");
        println!("    Remainder Layout (Iteration Space): {:?}", remainder.source().volume_extent());
    } else {
        println!("    FAILURE: Hardware Primitive does not fit.");
    }

    // --- STEP 6: Structural Judgments (Aliasing) ---
    // Create a broadcasting layout (e.g., [10] -> [1])
    let s10 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(10), None)]);
    let s1 = Space::new(vec![Factor::new(Kind::Logical, Extent::Constant(1), None)]);
    let broadcast = Expression::Broadcast(s10, s1);
    
    println!("[6] Structural Judgments:");
    println!("    Is Broadcast injective? {:?}", broadcast.is_injective());
    println!("    Is Broadcast aliasing?  {:?}", broadcast.is_aliasing(&valuation));

    println!("\n=== Walkthrough Complete ===");
}
