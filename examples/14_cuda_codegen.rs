use ftpl::*;

fn main() {
    println!("// === FTPL Generated CUDA Kernel ===\n");
    let valuation = Valuation::new();

    // 1. Define the Layout: 32x32 Matrix with Bank-Conflict-Free Swizzling
    // Construct using high-level Layout API and ergonomic space
    let mut swizzle_mat = vec![vec![0; 10]; 10];
    for i in 0..10 { swizzle_mat[i][i] = 1; }
    swizzle_mat[0][5] = 1; 

    let final_layout = Layout::row_major((32, 32))
        .transpose()
        .swizzle(swizzle_mat);

    // 2. Generate the Indexing Math
    let inputs = vec![ScalarExpr::Input(0), ScalarExpr::Input(1)]; // row, col
    let addr = final_layout.lower(&valuation, inputs);
    let cuda_idx = viz::cuda::to_cuda(&addr.0[0].clone().simplify(), &["row", "col"]);

    // 3. Output the Kernel
    let kernel = format!(r#"
__global__ void swizzled_transpose_kernel(float* out, float* in) {{
    int col = threadIdx.x;
    int row = threadIdx.y;
    __shared__ float tile[1024];
    int in_idx = row * 32 + col;
    tile[in_idx] = in[in_idx];
    __syncthreads();

    int out_idx_swizzled = {address_math};
    out[in_idx] = tile[out_idx_swizzled];
}}
"#, address_math = cuda_idx);

    println!("{}", kernel);
}
