use ftpl::*;

fn main() {
    println!("// === FTPL Generated GEMM CUDA Kernel ===\n");
    let valuation = Valuation::new();

    // 1. Setup Matrix Dimensions
    let (m, n, k) = (32, 32, 32);

    // 2. Define the 3D Iteration Space [M, N, K]
    let iter_space = Space::from((m, n, k));

    // 3. Define Layout for A: [M, K]
    // A Row-Major layout [M, K] has source (M, K).
    // We want a layout with source (M, N, K) that computes the same offset.
    // L_final = L_row_major o Reshape((M, K) -> (M, 1, K)) o Broadcast((M, 1, K) -> (M, N, K))
    
    let l_a_base = Layout::row_major((m, k));
    let l_a_reshaped = l_a_base.reshape(Space::from((m, 1, k)));
    
    // Manual composition for Broadcast (since it's not a fluent method yet)
    let l_a = Layout::new(Expression::Composition(
        Box::new(Expression::Broadcast(iter_space.clone(), Space::from((m, 1, k)))),
        Box::new(l_a_reshaped.expr)
    ));

    // 4. Define Layout for B: [K, N]
    let l_b_base = Layout::row_major((k, n));
    let l_b_reshaped = l_b_base.reshape(Space::from((1, n, k)));
    let l_b = Layout::new(Expression::Composition(
        Box::new(Expression::Broadcast(iter_space.clone(), Space::from((1, n, k)))),
        Box::new(l_b_reshaped.expr)
    ));

    // 5. Generate address math
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1), ScalarExpr::Input(2)];
    let addr_a = l_a.lower(&valuation, inputs.clone());
    let addr_b = l_b.lower(&valuation, inputs);

    if addr_a.0.is_empty() || addr_b.0.is_empty() {
        println!("// ERROR: Codegen failed. A_src_rank={}, B_src_rank={}", l_a.source().rank(), l_b.source().rank());
        return;
    }

    let cuda_a = viz::cuda::to_cuda(&addr_a.0[0].clone().simplify(), &["row", "col", "k"]);
    let cuda_b = viz::cuda::to_cuda(&addr_b.0[0].clone().simplify(), &["row", "col", "k"]);

    // 6. Generate the full CUDA Kernel
    let kernel = format!(r#"
__global__ void gemm_kernel(float* C, float* A, float* B) {{
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    float acc = 0.0f;
    for (int k = 0; k < {k_dim}; k++) {{
        int idx_a = {addr_a_math};
        int idx_b = {addr_b_math};
        acc += A[idx_a] * B[idx_b];
    }}
    C[row * {n_dim} + col] = acc;
}}
"#, k_dim = k, n_dim = n, addr_a_math = cuda_a, addr_b_math = cuda_b);

    println!("{}", kernel);
}
