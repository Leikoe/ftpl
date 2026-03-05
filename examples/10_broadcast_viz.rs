use ftpl::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let valuation = Valuation::new();

    // --- SCENARIO 1: Replicating a Row (Vertical Stripes) ---
    // Logical: [8, 8], Physical Storage: [1, 8]
    // The single row is repeated 8 times vertically.
    // Every cell in a column 'w' maps to the same offset 'w'.
    let logical_8x8 = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(8), Some("W".to_string())),
    ]);
    let physical_row = Space::new(vec![
        Factor::new(Kind::Storage, Extent::Constant(1), Some("H".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(8), Some("W".to_string())),
    ]);
    
    let broadcast_v = Expression::Broadcast(logical_8x8.clone(), physical_row);
    let svg1 = viz::render_svg(&broadcast_v, &valuation);
    
    let mut file = File::create("broadcast_vertical_stripes.svg").unwrap();
    file.write_all(svg1.as_bytes()).unwrap();
    println!("Saved broadcast_vertical_stripes.svg");

    // --- SCENARIO 2: Replicating a Column (Horizontal Stripes) ---
    // Logical: [8, 8], Physical Storage: [8, 1]
    // The single column is repeated 8 times horizontally.
    // Every cell in a row 'h' maps to the same offset 'h'.
    let physical_col = Space::new(vec![
        Factor::new(Kind::Storage, Extent::Constant(8), Some("H".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(1), Some("W".to_string())),
    ]);
    
    let broadcast_h = Expression::Broadcast(logical_8x8.clone(), physical_col);
    let svg2 = viz::render_svg(&broadcast_h, &valuation);
    
    let mut file = File::create("broadcast_horizontal_stripes.svg").unwrap();
    file.write_all(svg2.as_bytes()).unwrap();
    println!("Saved broadcast_horizontal_stripes.svg");

    // --- SCENARIO 3: Scalar Expansion ---
    // Logical: [8, 8], Physical Storage: [1, 1]
    // Every single cell in the 8x8 grid maps to the same memory address (Offset 0).
    let physical_1x1 = Space::new(vec![
        Factor::new(Kind::Storage, Extent::Constant(1), Some("H".to_string())),
        Factor::new(Kind::Storage, Extent::Constant(1), Some("W".to_string())),
    ]);
    
    let scalar_broadcast = Expression::Broadcast(logical_8x8, physical_1x1);
    let svg3 = viz::render_svg(&scalar_broadcast, &valuation);
    
    let mut file = File::create("broadcast_scalar.svg").unwrap();
    file.write_all(svg3.as_bytes()).unwrap();
    println!("Saved broadcast_scalar.svg");

    println!("\nANALYSIS:");
    println!("  - In the SVGs, cells with the SAME color/label represent ALIASED data.");
    println!("  - broadcast_vertical_stripes.svg: Vertical stripes (each row replicates the same row data).");
    println!("  - broadcast_horizontal_stripes.svg: Horizontal stripes (each col replicates the same col data).");
    println!("  - broadcast_scalar.svg: Entire grid is one color (all cells share one element).");
}
