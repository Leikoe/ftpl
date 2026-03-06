use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // 1. Logical Space: 16x16 Matrix
    let logical = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(16), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(16), Some("W".to_string())),
    ]);

    // 2. Hardware Target:
    // We want a Warp (32 threads) where threads are arranged in a 4x8 grid.
    // Each thread owns a 4x2 tile (8 elements).
    let hw_target = Space::new(vec![
        // 4x8 Thread Grid (Execution)
        Factor::new(
            Kind::Execution,
            Extent::Constant(4),
            Some("ThreadH".to_string()),
        ),
        Factor::new(
            Kind::Execution,
            Extent::Constant(8),
            Some("ThreadW".to_string()),
        ),
        // 4x2 Local Tile (Storage)
        Factor::new(
            Kind::Storage,
            Extent::Constant(4),
            Some("LocalH".to_string()),
        ),
        Factor::new(
            Kind::Storage,
            Extent::Constant(2),
            Some("LocalW".to_string()),
        ),
    ]);

    // 3. Create the distribution: (H, W) -> (ThreadH, ThreadW, LocalH, LocalW)
    // We tile the matrix:
    //   H (16) -> ThreadH (4) x LocalH (4)
    //   W (16) -> ThreadW (8) x LocalW (2)
    // Then we permute the factors to (ThreadH, ThreadW, LocalH, LocalW)

    let tiled_space = Space::new(vec![
        Factor::new(
            Kind::Execution,
            Extent::Constant(4),
            Some("ThreadH".to_string()),
        ),
        Factor::new(
            Kind::Storage,
            Extent::Constant(4),
            Some("LocalH".to_string()),
        ),
        Factor::new(
            Kind::Execution,
            Extent::Constant(8),
            Some("ThreadW".to_string()),
        ),
        Factor::new(
            Kind::Storage,
            Extent::Constant(2),
            Some("LocalW".to_string()),
        ),
    ]);

    let tiling = Expression::Reshape(logical, tiled_space.clone());

    // Permute to (ThreadH, ThreadW, LocalH, LocalW) -> [0, 2, 1, 3]
    let distribution = Expression::Composition(
        Box::new(tiling),
        Box::new(Expression::Permute(tiled_space, vec![0, 2, 1, 3])),
    );

    // 4. Add a Swizzle (Binary Shadow)
    // Let's swizzle the ThreadID bits with the LocalOffset bits.
    // This matrix will swap bit 0 and bit 4 of the linearized target offset.
    // Linearized Target Offset = (ThreadH*8 + ThreadW)*8 + (LocalH*2 + LocalW)
    // Total bits = 2(TH) + 3(TW) + 2(LH) + 1(LW) = 8 bits.
    let mut matrix = vec![vec![0; 8]; 8];
    for i in 0..8 {
        matrix[i][i] = 1;
    }
    // Swap bit 0 (LW) and bit 3 (TW low bit)
    matrix[0][0] = 0;
    matrix[0][3] = 1;
    matrix[3][3] = 0;
    matrix[3][0] = 1;

    let swizzle = Expression::BinaryShadow(hw_target, matrix);

    // Final Mapping
    let mapping = Expression::Composition(Box::new(distribution), Box::new(swizzle));

    // 5. Render
    let svg = viz::render_svg(&mapping, &valuation);
    let mut file = File::create("complex_swizzled_mapping.svg").unwrap();
    file.write_all(svg.as_bytes()).unwrap();

    println!("Saved complex_swizzled_mapping.svg");
    println!("This visualization shows a 16x16 matrix distributed over a 4x8 thread grid.");
    println!("Each thread owns a 4x2 tile. We applied a bit-permute swizzle to the mapping.");
}
