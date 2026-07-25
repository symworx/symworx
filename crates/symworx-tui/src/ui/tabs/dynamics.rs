use ratatui::{
    layout::Rect,
    style::{
        Color,
        Style,
    },
    text::{
        Line,
        Span,
    },
    widgets::{
        Block,
        Borders,
        Padding,
        Paragraph,
    },
    Frame,
};

use crate::app::App;

pub fn render_dynamics_tab(frame: &mut Frame, app: &App, area: Rect) {
    if app.help_mode {
        let help = Paragraph::new(
            "Dynamics — RQA, cRQA, multiscale entropy\n\
             Close help:  Esc  or  Alt-?\n\n\
             \n\
             INPUT\n\n\
               Uses the current Explore series (process / peaks there first).\n\
               MSE is shown automatically from that series.\n\n\
             \n\
             RQA  /  cRQA\n\n\
               c                   open param editor (m, τ, radius)\n\
               Enter (in editor)   compute RQA + recurrence plot\n\
               ← → / ±             nudge radius\n\
               m / t               cycle embedding dim / delay\n\
               Esc (in editor)     cancel editor\n\
               r                   reset params + results (pin kept)\n\n\
               p                   pin current series as cRQA reference\n\
               x                   compute cRQA\n\
                                   (pinned ref vs current, or current vs reverse)\n\
               e                   export last RQA/cRQA metrics → data/\n\n\
             \n\
             NAV\n\n\
               Esc                 back to Explore\n\
               Ctrl+H              Home\n\
               Esc Esc / Ctrl+Q    quit (Esc-Esc at roots only)\n",
        )
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .title(" Help — Dynamics "),
        );
        frame.render_widget(help, area);
        return;
    }

    let block = Block::new()
        .title(" Dynamics — RQA (Recurrence Quantification Analysis) ")
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Color::Green);

    if app.pending_rqa {
        let txt = format!(
            "\n\nRQA/cRQA Parameter Editor (shared params)\n\n\
             m (embedding dim): {}     (press m to cycle)\n\
             tau (delay): {}           (press t to cycle)\n\
             radius: {:.2}             (← → adjust)\n\
             theiler: {}\n\n\
             Enter = compute RQA (auto-recurrence)\n\
             (outside: x = cRQA using ref or fallback)\n\
             Esc = cancel",
            app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius, app.rqa_params.theiler
        );
        frame.render_widget(Paragraph::new(txt).block(block), area);
        return;
    }

    if let Some(sig) = &app.loaded_signal {
        let mut content_lines: Vec<Line> = vec![
            Line::from(format!("Signal: {} ({} samples)", sig.name, sig.n_samples)),
            Line::from(format!(
                "Params: m={}  tau={}  radius={:.2}  theiler={}",
                app.rqa_params.m, app.rqa_params.tau, app.rqa_params.radius, app.rqa_params.theiler
            )),
            Line::from(""),
        ];

        // Show RQA or cRQA results
        if let Some(r) = &app.last_crqa {
            content_lines.push(Line::from("cRQA result (cross-recurrence):"));
            content_lines.push(Line::from(format!(
                "RR: {:.3}  DET: {:.3}  LAM: {:.3}",
                r.recurrence_rate, r.determinism, r.laminarity
            )));
            content_lines.push(Line::from(format!(
                "Lmax/Vmax: {}/{}   Lentr/TT: {:.2}/{:.2}",
                r.lmax, r.vmax, r.lentr, r.trapping_time
            )));
            content_lines.push(Line::from(""));
        } else if let Some(r) = &app.last_rqa {
            content_lines.push(Line::from(format!(
                "RR (recurrence rate): {:.3}",
                r.recurrence_rate
            )));
            content_lines.push(Line::from(format!(
                "DET (determinism)   : {:.3}",
                r.determinism
            )));
            content_lines.push(Line::from(format!(
                "LAM (laminarity)    : {:.3}",
                r.laminarity
            )));
            content_lines.push(Line::from(format!(
                "Lmax / Vmax         : {} / {}",
                r.lmax, r.vmax
            )));
            content_lines.push(Line::from(format!(
                "Lentr / TT          : {:.2} / {:.2}",
                r.lentr, r.trapping_time
            )));
            content_lines.push(Line::from(""));
        } else {
            content_lines.push(Line::from(
                "No RQA result yet. Press 'c' to edit params & compute.",
            ));
        }

        // Reference for cRQA
        if let Some((ref_name, _)) = &app.reference_series {
            content_lines.push(Line::from(format!("Reference for cRQA: {}", ref_name)));
        }

        // Entropy measures (SampEn + MSE at several scales)
        let stats = crate::app::compute_basic_stats(&sig.current);
        let r_se = 0.2 * stats.std.max(0.01);
        let se = symworx_dynamics::sample_entropy(&sig.current, 2, r_se);
        content_lines.push(Line::from(format!(
            "Sample Entropy (m=2, r=0.2σ): {:.4}",
            se
        )));

        let mse = symworx_dynamics::multiscale_entropy(&sig.current, 6, 2, r_se);
        let mse_str: Vec<String> = mse
            .iter()
            .enumerate()
            .map(|(i, v)| format!("s{}:{:.3}", i + 1, v))
            .collect();
        content_lines.push(Line::from(format!(
            "Multiscale Entropy (scales 1-6): {}",
            mse_str.join("  ")
        )));
        content_lines.push(Line::from(""));

        // Improved recurrence plot preview (colored unicode)
        if app.last_rqa.is_some() || app.last_crqa.is_some() {
            content_lines.push(Line::from(Span::raw(
                "Recurrence Plot preview (█ recurrent, · elsewhere; downsampled):",
            )));
            let rp_lines = render_styled_rp_preview(
                &sig.current,
                app.rqa_params.m,
                app.rqa_params.tau,
                app.rqa_params.radius,
                app.rqa_params.theiler,
            );
            content_lines.extend(rp_lines);
            content_lines.push(Line::from(""));
            content_lines.push(Line::from(
                "Keys: c=edit/compute RQA  x=cRQA  p=pin ref  e=export  r=reset",
            ));
        } else {
            content_lines.push(Line::from(
                "Compute (c) to see recurrence structure preview + MSE.",
            ));
        }

        let content = Paragraph::new(content_lines).block(block);
        frame.render_widget(content, area);
    } else {
        let content = Paragraph::new(
            "\n\nLoad a signal via BioSym path / Import (or Ctrl+G) first.\n\
             Then use this tab for RQA/cRQA + entropy: c = params & compute\n\
             Results: RR/DET/LAM + multiscale entropy from symworx-dynamics.",
        )
        .centered();
        frame.render_widget(content.block(block), area);
    }
}

/// Downsampled styled RP preview. Returns Lines using colored Spans for better visibility.
/// Recurrent points in bright green; non-recurrent dim or spaces. Uses symworx-dynamics.
fn render_styled_rp_preview(
    series: &[f64],
    m: usize,
    tau: usize,
    radius: f64,
    theiler: usize,
) -> Vec<Line<'static>> {
    use symworx_dynamics::RecurrencePlot;

    let max_pts = 42usize; // a bit denser than before
    let n = series.len();
    if n < (m + 1) * tau + 2 {
        return vec![Line::from("(series too short for RP preview)")];
    }

    let step = (n / max_pts).max(1);
    let sub: Vec<f64> = series.iter().step_by(step).take(max_pts).copied().collect();
    if sub.len() < m {
        return vec![Line::from("(subsample too small)")];
    }

    let rp = RecurrencePlot::from_series(&sub, m, tau, radius, theiler);
    let mat = &rp.matrix;
    let side = mat.nrows().min(40);

    let mut out = Vec::with_capacity(side);
    let rec_style = Style::default()
        .fg(Color::Green)
        .add_modifier(ratatui::style::Modifier::BOLD);
    let non_style = Style::default().fg(Color::DarkGray);

    for i in 0..side {
        let mut spans = Vec::new();
        for j in 0..side {
            let cell = mat[[i, j]];
            if cell {
                spans.push(Span::styled("██", rec_style));
            } else {
                spans.push(Span::styled("· ", non_style));
            }
        }
        out.push(Line::from(spans));
    }
    out
}
