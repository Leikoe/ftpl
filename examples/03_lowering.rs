use ftpl::*;

fn main() {
    println!("--- Example 03: Constructing Row vs Column Major ---");
    let valuation = Valuation::new();

    // Define a 2D Space [H=4, W=8]
    let s = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);

    // 1. ROW-MAJOR: right-most dimension is fastest
    let l_row = Expression::Linearize(s.clone());
    let addr_row = l_row.lower(&valuation, vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]);
    println!(
        "Row-Major CUDA Math:    {}",
        viz::cuda::to_cuda(&addr_row.0[0].clone().simplify(), &["H", "W"])
    );

    // 2. COLUMN-MAJOR: left-most dimension is fastest
    let transpose = Expression::Permute(s.clone(), vec![1, 0]);
    let s_transposed = s.permute(&[1, 0]);

    let l_col = Expression::Composition(
        Box::new(transpose),
        Box::new(Expression::Linearize(s_transposed)),
    );

    let addr_col = l_col.lower(&valuation, vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]);
    println!(
        "Column-Major CUDA Math: {}",
        viz::cuda::to_cuda(&addr_col.0[0].clone().simplify(), &["H", "W"])
    );

    println!("\nVerification at (1, 0):");
    println!(
        "  Row-Major Offset: {}",
        l_row.apply(&valuation, &[1, 0]).unwrap()[0]
    );
    println!(
        "  Col-Major Offset: {}",
        l_col.apply(&valuation, &[1, 0]).unwrap()[0]
    );
}
