use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // 1. Logical Space: 16x16 Matrix
    let logical = Space::from((16, 16));

    // 2. Hardware Target
    let tiled_space = Space::new(vec![
        Factor::new(Kind::Execution, Extent::Constant(4), Some("ThreadH".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(4), Some("LocalH".to_string())),
        Factor::new(Kind::Execution, Extent::Constant(8), Some("ThreadW".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(2), Some("LocalW".to_string())),
    ]);
    
    // Fluent composition
    let mapping = Layout::identity(logical)
        .reshape(tiled_space)
        .permute(vec![0, 2, 1, 3]);

    // 3. Add a Swizzle (Binary Shadow)
    let mut matrix = vec![vec![0; 8]; 8];
    for i in 0..8 { matrix[i][i] = 1; }
    matrix[0][3] = 1; matrix[3][0] = 1;
    
    let mapping_swizzled = mapping.swizzle(matrix);

    // 4. Render
    let svg = viz::render_svg(&mapping_swizzled.expr, &valuation);
    let mut file = File::create("complex_swizzled_mapping.svg").unwrap();
    file.write_all(svg.as_bytes()).unwrap();
    println!("Saved complex_swizzled_mapping.svg");
}
