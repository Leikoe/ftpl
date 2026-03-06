use crate::layout::ScalarExpr;

/// Transpiles an FTPL ScalarExpr to CUDA C++ code.
pub fn to_cuda(expr: &ScalarExpr, input_names: &[&str]) -> String {
    match expr {
        ScalarExpr::Input(i) => {
            if *i < input_names.len() {
                input_names[*i].to_string()
            } else {
                format!("i{}", i)
            }
        }
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
