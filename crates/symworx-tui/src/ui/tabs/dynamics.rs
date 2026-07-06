use ratatui::{
    layout::Rect,
    style::Color,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render_dynamics_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Dynamics help (M-? or Esc to close)\n\n\
             • c : open RQA param editor (m/tau/radius)\n\
             • r : reset RQA params\n\
             • (in editor) ←→ m t Enter Esc\n\n\
             Requires loaded signal from BioSym (Import/Explore).\n\
             Uses symworx-dynamics::rqa for recurrence quantification.",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title(" Help — Dynamics "),
        );
        frame.render_widget(help, area);
        return;
    }

    let block = Block::new()
        .title(" Dynamics — RQA (Recurrence Quantification Analysis) ")
        .borders(Borders::ALL)
        .border_style(Color::Green);

    if app.pending_rqa {
        let txt = format!(
            "\n\nRQA Parameter Editor\n\n\
             m (embedding dim): {}     (press m to cycle)\n\
             tau (delay): {}           (press t to cycle)\n\
             radius: {:.2}             (← → adjust)\n\
             theiler: {}\n\n\
             Enter = compute using symworx-dynamics::rqa\n\
             Esc = cancel",
            app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius, app.rqa_params.theiler
        );
        frame.render_widget(Paragraph::new(txt).block(block), area);
        return;
    }

    if let Some(sig) = &app.loaded_signal {
        let mut lines: Vec<String> = vec![
            format!("Signal: {} ({} samples)", sig.name, sig.n_samples),
            format!(
                "Params: m={}  tau={}  radius={:.2}  theiler={}",
                app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius, app.rqa_params.theiler
            ),
            "".into(),
        ];

        if let Some(r) = &app.last_rqa {
            lines.push(format!("RR (recurrence rate): {:.3}", r.recurrence_rate));
            lines.push(format!("DET (determinism)   : {:.3}", r.determinism));
            lines.push(format!("LAM (laminarity)    : {:.3}", r.laminarity));
            lines.push(format!("Lmax / Vmax         : {} / {}", r.lmax, r.vmax));
            lines.push(format!(
                "Lentr / TT          : {:.2} / {:.2}",
                r.lentr, r.trapping_time
            ));
            lines.push("".into());
        } else {
            lines.push("No RQA result yet. Press 'c' to edit params & compute.".into());
        }

        // Sample entropy (additional measure) — computed on demand from current signal
        if let Some(sig) = &app.loaded_signal {
            let se = symworx_dynamics::sample_entropy(
                &sig.current,
                2,
                0.2 * crate::app::compute_basic_stats(&sig.current).std.max(0.01),
            );
            lines.push(format!("Sample Entropy (m=2, r=0.2σ): {:.4}", se));
        }

        // Unicode block recurrence plot preview (downsampled)
        if let Some(sig) = &app.loaded_signal {
            if app.last_rqa.is_some() {
                let preview = render_simple_unicode_rp(
                    &sig.current,
                    app.rqa_params.m,
                    app.rqa_params.tau,
                    app.rqa_params.radius,
                    app.rqa_params.theiler,
                );
                lines.push(
                    "Recurrence Plot (downsampled unicode blocks — dense = recurrent):".into(),
                );
                lines.push(preview);
                lines.push(
                    "Full resolution: use 'e' to export CSV of recurrence matrix or metrics."
                        .into(),
                );
            }
        } else {
            lines.push("Compute to see recurrence structure preview.".into());
        }

        let content = Paragraph::new(lines.join("\n")).block(block);
        frame.render_widget(content, area);
    } else {
        let content = Paragraph::new(
            "\n\nLoad a signal via BioSym path / Import (or Ctrl+G) first.\n\
             Then use this tab for RQA: c = set params & compute (m/tau/radius)\n\
             Results include RR, DET, LAM, line lengths per symworx-dynamics.",
        )
        .centered();
        frame.render_widget(content.block(block), area);
    }
}

/// Downsample + render a tiny unicode RP preview using blocks.
/// Uses symworx-dynamics under the hood for the matrix (recomputes small).
fn render_simple_unicode_rp(
    series: &[f64],
    m: usize,
    tau: usize,
    radius: f64,
    theiler: usize,
) -> String {
    use symworx_dynamics::RecurrencePlot;
    // Limit for UI perf + readability
    let max_pts = 36usize;
    let n = series.len();
    if n < (m + 1) * tau + 2 {
        return "(series too short for preview)".to_string();
    }

    // Subsample the series for the preview (take evenly)
    let step = (n / max_pts).max(1);
    let sub: Vec<f64> = series.iter().step_by(step).take(max_pts).copied().collect();
    if sub.len() < m {
        return "(subsample too small)".to_string();
    }

    let rp = RecurrencePlot::from_series(&sub, m, tau, radius, theiler);
    let mat = &rp.matrix;
    let side = mat.nrows().min(32);

    let mut out = String::new();
    for i in 0..side {
        for j in 0..side {
            let cell = mat[[i, j]];
            // Unicode blocks
            let ch = if cell { "██" } else { "  " };
            out.push_str(ch);
        }
        out.push('\n');
    }
    out
}
