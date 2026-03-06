use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // 1. Row-Major [4, 8]
    let s4_8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let l_row_major = Expression::Linearize(s4_8.clone());
    let svg_row_major = viz::render_svg(&l_row_major, &valuation);

    let mut file = File::create("row_major.svg").unwrap();
    file.write_all(svg_row_major.as_bytes()).unwrap();
    println!("Saved row_major.svg");

    // 2. Transposed [8, 4] -> Reshaped back to [4, 8]
    // This is the "broken contiguity" case from Example 06.
    let s8_4 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(4), Some("H".to_string())),
    ]);
    let transpose = Expression::Permute(s8_4.clone(), vec![1, 0]);
    let reshape = Expression::Reshape(s4_8.clone(), s8_4.clone());

    let l_transposed = Expression::Composition(
        Box::new(reshape),
        Box::new(Expression::Composition(
            Box::new(transpose),
            Box::new(l_row_major),
        )),
    );

    let svg_transposed = viz::render_svg(&l_transposed, &valuation);
    let mut file = File::create("transposed_reshaped.svg").unwrap();
    file.write_all(svg_transposed.as_bytes()).unwrap();
    println!("Saved transposed_reshaped.svg");
}
