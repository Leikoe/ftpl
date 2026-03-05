use ftpl::*;

fn main() {
    println!("--- Example 04: Hardware Primitive Fit ---");

    // 1. Hardware Primitive: A 16x16 fragment instruction (e.g., Tensor Core)
    let frag_16x16 = Space::new(vec![
        Factor::new(Kind::Fragment, Extent::Constant(16), None),
        Factor::new(Kind::Fragment, Extent::Constant(16), None),
    ]);
    let hardware_primitive = Expression::Linearize(frag_16x16.clone());

    // 2. Program Layout: A larger 32x32 tensor, tiled as (16, 16) fragments
    // program = primitive x 2x2 tiling
    let tile_2x2 = Space::new(vec![
        Factor::new(Kind::Tile, Extent::Constant(2), None),
        Factor::new(Kind::Tile, Extent::Constant(2), None),
    ]);
    let program_layout = Expression::Product(
        Box::new(hardware_primitive.clone()),
        Box::new(Expression::Identity(tile_2x2))
    );

    // 3. Verification: Can we fit the 16x16 primitive onto the 32x32 layout?
    // This is checking if the primitive "divides" into the layout.
    if let Some(remainder) = program_layout.left_div(hardware_primitive) {
        println!("SUCCESS: Primitive Fits!");
        println!("Remainder Layout (the 2x2 loop space): {:?}", remainder.source().volume_extent());
    } else {
        println!("FAILURE: Primitive Does Not Fit.");
    }
}
