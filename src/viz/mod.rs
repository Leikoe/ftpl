pub mod cuda;

use crate::core::{Kind, Space, Valuation};
use crate::layout::{AsLayout, Expression, Layout};

/// Renders a 2D layout as an SVG grid.
/// Colors cells by the Execution resource (e.g. ThreadID) and labels with Storage offset.
pub fn render_svg(layout: &Expression, valuation: &Valuation) -> String {
    let src = layout.source();
    let tgt = layout.target();

    if src.factors.len() != 2 {
        return "SVG Rendering currently only supports 2D source spaces (H, W)".to_string();
    }

    let h_extent = valuation.get_extent(&src.factors[0].extent).unwrap_or(1);
    let w_extent = valuation.get_extent(&src.factors[1].extent).unwrap_or(1);

    let cell_size = 60;
    let width = w_extent * cell_size;
    let height = h_extent * cell_size;

    // Identify which target factors are Execution vs Storage
    let mut exec_indices = Vec::new();
    let mut storage_indices = Vec::new();
    for (i, f) in tgt.factors.iter().enumerate() {
        match f.kind {
            Kind::Execution => exec_indices.push(i),
            Kind::Storage | Kind::Other(_) => storage_indices.push(i),
            _ => {}
        }
    }

    let mut svg = format!(
        "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\n",
        width, height
    );

    for h in 0..h_extent {
        for w in 0..w_extent {
            let x = w * cell_size;
            let y = h * cell_size;

            let out = layout
                .apply(valuation, &[h, w])
                .unwrap_or_else(|| vec![0; tgt.factors.len()]);

            // 1. Calculate Execution Unit ID for coloring
            let mut exec_id = 0;
            let mut has_exec = false;
            let mut stride = 1;
            for &idx in exec_indices.iter().rev() {
                exec_id += out[idx] * stride;
                stride *= valuation.get_extent(&tgt.factors[idx].extent).unwrap_or(1);
                has_exec = true;
            }

            // 2. Calculate Storage Offset for text
            // We linearize the storage factors to get a single global offset
            let mut storage_val = 0;
            let mut storage_stride = 1;
            for &idx in storage_indices.iter().rev() {
                storage_val += out[idx] * storage_stride;
                storage_stride *= valuation.get_extent(&tgt.factors[idx].extent).unwrap_or(1);
            }

            // 3. Generate color
            // If has execution units, color by ID. Otherwise, color by offset (heatmap).
            let color = if has_exec {
                let hue = (exec_id * 137) % 360;
                format!("hsl({}, 70%, 80%)", hue)
            } else {
                let max_vol = tgt
                    .volume_extent()
                    .try_eval(&valuation.variables)
                    .unwrap_or(1);
                let intensity = (storage_val as f64 / max_vol as f64 * 200.0) as u8;
                format!("rgb({}, {}, 255)", 255 - intensity, 255 - intensity)
            };

            svg.push_str(&format!(
                "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"black\" />\n",
                x, y, cell_size, cell_size, color
            ));

            // 4. Context-Aware Label
            let label = if has_exec {
                format!("T{}:{}", exec_id, storage_val)
            } else {
                format!("{}", storage_val)
            };

            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" font-size=\"10\" text-anchor=\"middle\" fill=\"black\">{}</text>\n",
                x + cell_size / 2, y + cell_size / 2 + 5, label
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}
