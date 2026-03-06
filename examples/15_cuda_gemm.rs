use ftpl::*;

/// A transpiler that converts FTPL ScalarExpr to CUDA C++ code.
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
            let mut xor_terms = Vec::new();
            for i in 0..m.len() {
                for j in 0..m[0].len() {
                    if i != j && m[i][j] == 1 {
                        xor_terms.push(format!("(({} >> {}) & 1)", val_str, j));
                    }
                }
            }
            if xor_terms.is_empty() {
                val_str
            } else {
                format!("({} ^ ({}))", val_str, xor_terms.join(" ^ "))
            }
        }
        _ => "0".to_string(),
    }
}

fn main() {
    println!("// === FTPL Generated GEMM CUDA Kernel ===\n");
    let valuation = Valuation::new();

    // 1. Setup Matrix Dimensions
    let (m, n, k) = (32, 32, 32);

    // 2. Define the 3D Iteration Space [M, N, K]
    // Indices: i0=M (row), i1=N (col), i2=K (reduction)
    let iter_space = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(m), Some("i".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(n), Some("j".to_string())),
        Factor::new(Kind::Logical, Extent::Constant(k), Some("k".to_string())),
    ]);

    // 3. Define Layout for A: [M, K]
    // Viewed from 3D: [M, N, K] -> [M, K] (Broadcasting N)
    let s_a = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(m), None),
        Factor::new(Kind::Logical, Extent::Constant(k), None),
    ]);
    let l_a_base = Expression::Linearize(s_a);
    // Broadcast N: (M, N, K) -> (M, K)
    let l_a = Expression::Composition(
        Box::new(Expression::Broadcast(
            iter_space.clone(),
            Space::new(vec![
                Factor::new(Kind::Logical, Extent::Constant(m), None),
                Factor::new(Kind::Logical, Extent::Constant(1), None),
                Factor::new(Kind::Logical, Extent::Constant(k), None),
            ]),
        )),
        Box::new(Expression::Reshape(
            Space::new(vec![
                Factor::new(Kind::Logical, Extent::Constant(m), None),
                Factor::new(Kind::Logical, Extent::Constant(1), None),
                Factor::new(Kind::Logical, Extent::Constant(k), None),
            ]),
            l_a_base.source(),
        )),
    );
    let l_a_final = Expression::Composition(Box::new(l_a), Box::new(l_a_base));

    // 4. Define Layout for B: [K, N] (User specified B.T.reshape(1, N, N))
    // We map (M, N, K) -> (K, N)
    let s_b = Space::new(vec![
        Factor::new(Kind::Logical, Extent::Constant(k), None),
        Factor::new(Kind::Logical, Extent::Constant(n), None),
    ]);
    let l_b_base = Expression::Linearize(s_b);
    let l_b = Expression::Composition(
        Box::new(Expression::Broadcast(
            iter_space.clone(),
            Space::new(vec![
                Factor::new(Kind::Logical, Extent::Constant(1), None),
                Factor::new(Kind::Logical, Extent::Constant(n), None),
                Factor::new(Kind::Logical, Extent::Constant(k), None),
            ]),
        )),
        Box::new(Expression::Reshape(
            Space::new(vec![
                Factor::new(Kind::Logical, Extent::Constant(1), None),
                Factor::new(Kind::Logical, Extent::Constant(n), None),
                Factor::new(Kind::Logical, Extent::Constant(k), None),
            ]),
            l_b_base.source(),
        )),
    );
    let l_b_final = Expression::Composition(Box::new(l_b), Box::new(l_b_base));

    // 5. Generate address math
    // Inputs: i0=row, i1=col, i2=k
    let inputs = vec![
        ScalarExpr::Input(0),
        ScalarExpr::Input(1),
        ScalarExpr::Input(2),
    ];
    let (addr_a, _) = l_a_final.lower(&valuation, inputs.clone());
    let (addr_b, _) = l_b_final.lower(&valuation, inputs);

    let cuda_a = to_cuda(&addr_a[0].clone().simplify(), &["row", "col", "k"]);
    let cuda_b = to_cuda(&addr_b[0].clone().simplify(), &["row", "col", "k"]);

    // 6. Generate the full CUDA Kernel
    let kernel = format!(
        r#"
__global__ void gemm_kernel(float* C, float* A, float* B) {{
    // Each thread (row, col) computes one element of C
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    int row = blockIdx.y * blockDim.y + threadIdx.y;

    float acc = 0.0f;

    // The reduction loop over K
    for (int k = 0; k < {k_dim}; k++) {{
        // FTPL-Generated Address Math
        int idx_a = {addr_a_math};
        int idx_b = {addr_b_math};

        acc += A[idx_a] * B[idx_b];
    }}

    // Store to C
    C[row * {n_dim} + col] = acc;
}}
"#,
        k_dim = k,
        n_dim = n,
        addr_a_math = cuda_a,
        addr_b_math = cuda_b
    );

    println!("{}", kernel);
}
