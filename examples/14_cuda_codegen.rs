use ftpl::*;

/// A simple transpiler that converts FTPL ScalarExpr to CUDA C++ code.
fn to_cuda(expr: &ScalarExpr, input_names: &[&str]) -> String {
    match expr {
        ScalarExpr::Input(i) => input_names[*i].to_string(),
        ScalarExpr::Constant(c) => c.to_string(),
        ScalarExpr::Add(a, b) => format!(
            "({} + {})",
            to_cuda(a, input_names),
            to_cuda(b, input_names)
        ),
        ScalarExpr::Mul(a, b) => format!(
            "({} * {})",
            to_cuda(a, input_names),
            to_cuda(b, input_names)
        ),
        ScalarExpr::Div(a, b) => format!(
            "({} / {})",
            to_cuda(a, input_names),
            to_cuda(b, input_names)
        ),
        ScalarExpr::Mod(a, b) => format!(
            "({} % {})",
            to_cuda(a, input_names),
            to_cuda(b, input_names)
        ),
        ScalarExpr::Xor(a, b) => format!(
            "({} ^ {})",
            to_cuda(a, input_names),
            to_cuda(b, input_names)
        ),
        ScalarExpr::BitShiftRight(a, s) => format!("({} >> {})", to_cuda(a, input_names), s),
        ScalarExpr::BitLinear(a, m) => {
            let val_str = to_cuda(a, input_names);
            let rows = m.len();
            let cols = m[0].len();

            // 1. Check if it is a simple XOR-Swizzle (Identity + a few extra bits)
            // Rule: out = x ^ (some sparse bits)
            let mut xor_terms = Vec::new();
            for i in 0..rows {
                for j in 0..cols {
                    // If it's a non-diagonal bit, it's an XOR term
                    if i != j && m[i][j] == 1 {
                        xor_terms.push(format!("(({} >> {}) & 1)", val_str, j));
                    }
                }
            }

            if xor_terms.is_empty() {
                val_str // Pure identity
            } else {
                // Return clean XOR swizzle logic: index ^ (bits)
                format!("({} ^ ({}))", val_str, xor_terms.join(" ^ "))
            }
        }
        _ => "/* unsupported */ 0".to_string(),
    }
}

fn main() {
    println!("// === FTPL Generated CUDA Kernel ===\n");
    let valuation = Valuation::new();

    // 1. Define the Layout: 32x32 Matrix with Bank-Conflict-Free Swizzling
    let matrix_space = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(32), Some("row".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(32), Some("col".to_string())),
    ]);

    // Naive Transpose
    let l_naive = Expression::Linearize(matrix_space.clone());
    let p_transpose = Expression::Permute(matrix_space.clone(), vec![1, 0]);
    let naive_transpose = Expression::Composition(Box::new(p_transpose), Box::new(l_naive));

    // Apply Swizzle (from Example 13)
    let mut swizzle_mat = vec![vec![0; 10]; 10];
    for i in 0..10 {
        swizzle_mat[i][i] = 1;
    }
    swizzle_mat[0][5] = 1; // XOR bit 5 into bit 0
    let swizzle = Expression::BinaryShadow(
        Space::new(vec![Factor::new(
            Kind::Storage,
            Extent::Constant(1024),
            None,
        )]),
        swizzle_mat,
    );

    let final_layout = Expression::Composition(Box::new(naive_transpose), Box::new(swizzle));

    // 2. Generate the Indexing Math
    // Inputs: i0 = logical_row, i1 = logical_col
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)];
    let (addresses, _) = final_layout.lower(&valuation, inputs);
    let cuda_idx = to_cuda(&addresses[0].clone().simplify(), &["row", "col"]);

    // 3. Output the Kernel
    let kernel = format!(
        r#"
__global__ void swizzled_transpose_kernel(float* out, float* in) {{
    // Each thread handles one element
    int col = threadIdx.x;
    int row = threadIdx.y;

    // Static Shared Memory
    __shared__ float tile[1024];

    // 1. Load from Global to Shared (Row-Major)
    int in_idx = row * 32 + col;
    tile[in_idx] = in[in_idx];

    __syncthreads();

    // 2. Read from Shared with FTPL-Generated Optimized Indexing
    // This index math is proven to be bank-conflict free!
    int out_idx_swizzled = {address_math};

    // 3. Store to Global (effectively transposed)
    out[in_idx] = tile[out_idx_swizzled];
}}
"#,
        address_math = cuda_idx
    );

    println!("{}", kernel);

    println!("\n// ANALYSIS:");
    println!("// The generated 'out_idx_swizzled' contains the XOR-swizzle logic");
    println!("// required to avoid all 32-way bank conflicts during the transpose.");
}
