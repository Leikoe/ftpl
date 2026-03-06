use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // --- SCENARIO 1: Replicating a Row (Vertical Stripes) ---
    let logical_8x8 = Space::from((8, 8));
    let physical_row = Space::new(vec![
        Factor::new(Kind::Storage, Extent::Constant(1), Some("H".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(8), Some("W".to_string())),
    ]);
    
    let broadcast_v = Layout::new(Expression::Broadcast(logical_8x8.clone(), physical_row));
    let svg1 = viz::render_svg(&broadcast_v.expr, &valuation);
    
    let addr1 = broadcast_v.lower(&valuation, vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]);
    println!("Row Replication (Vertical Stripes) CUDA: {}", viz::cuda::to_cuda(&addr1.0[0].clone().simplify(), &["h", "w"]));

    let mut file = File::create("broadcast_vertical_stripes.svg").unwrap();
    file.write_all(svg1.as_bytes()).unwrap();
    println!("Saved broadcast_vertical_stripes.svg");

    // --- SCENARIO 2: Replicating a Column (Horizontal Stripes) ---
    let physical_col = Space::new(vec![
        Factor::new(Kind::Storage, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(1), Some("W".to_string())),
    ]);
    
    let broadcast_h = Layout::new(Expression::Broadcast(logical_8x8.clone(), physical_col));
    let svg2 = viz::render_svg(&broadcast_h.expr, &valuation);
    
    let addr2 = broadcast_h.lower(&valuation, vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]);
    println!("Col Replication (Horizontal Stripes) CUDA: {}", viz::cuda::to_cuda(&addr2.0[0].clone().simplify(), &["h", "w"]));

    let mut file = File::create("broadcast_horizontal_stripes.svg").unwrap();
    file.write_all(svg2.as_bytes()).unwrap();
    println!("Saved broadcast_horizontal_stripes.svg");

    // --- SCENARIO 3: Scalar Expansion ---
    let physical_1x1 = Space::new(vec![
        Factor::new(Kind::Storage, Extent::Constant(1), None),
        Factor::new(Kind::Storage, Extent::Constant(1), None),
    ]);
    
    let scalar_broadcast = Layout::new(Expression::Broadcast(logical_8x8, physical_1x1));
    let svg3 = viz::render_svg(&scalar_broadcast.expr, &valuation);
    
    let addr3 = scalar_broadcast.lower(&valuation, vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]);
    println!("Scalar Expansion CUDA: {}", viz::cuda::to_cuda(&addr3.0[0].clone().simplify(), &["h", "w"]));

    let mut file = File::create("broadcast_scalar.svg").unwrap();
    file.write_all(svg3.as_bytes()).unwrap();
    println!("Saved broadcast_scalar.svg");
}
