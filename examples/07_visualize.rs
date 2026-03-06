use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // 1. Row-Major [4, 8]
    let l_row_major = Layout::row_major((4, 8));
    let svg_row_major = viz::render_svg(&l_row_major.expr, &valuation);
    
    let mut file = File::create("row_major.svg").unwrap();
    file.write_all(svg_row_major.as_bytes()).unwrap();
    println!("Saved row_major.svg");

    // 2. Transposed [8, 4] -> Reshaped back to [4, 8]
    let l_transposed = l_row_major
        .transpose()
        .reshape((4, 8));

    let svg_transposed = viz::render_svg(&l_transposed.expr, &valuation);
    let mut file = File::create("transposed_reshaped.svg").unwrap();
    file.write_all(svg_transposed.as_bytes()).unwrap();
    println!("Saved transposed_reshaped.svg");
}
