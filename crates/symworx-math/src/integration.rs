// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use ndarray::Array1;

/// Cumulative trapezoidal integration
#[inline]
pub fn cumtrapz(y: &[f64], dx: f64) -> Vec<f64> {
    let n = y.len();
    if n == 0 {
        return vec![];
    }

    let mut out = Vec::with_capacity(n);
    out.push(0.0);

    let mut acc = 0.0;
    for i in 1..n {
        acc += 0.5 * (y[i - 1] + y[i]) * dx;
        out.push(acc);
    }

    out
}

/// Single trapezoidal integral over the whole array.
#[inline]
pub fn trapz(y: &[f64], dx: f64) -> f64 {
    let n = y.len();
    if n < 2 {
        return 0.0;
    }

    let mut acc = 0.0;
    for i in 1..n {
        acc += 0.5 * (y[i - 1] + y[i]) * dx;
    }
    acc
}

// ODE Integration (RK4) - idiomatic, minimal deps

/// Perform a single 4th-order Runge-Kutta (RK4) step.
///
/// # Arguments
/// * `f` - Derivative function: f(t, y) -> dy/dt as Array1<f64>
/// * `t` - Current time
/// * `y` - Current state (Array1)
/// * `dt` - Time step size
///
/// Returns new state at t + dt.
pub fn rk4_step<F>(f: F, t: f64, y: &Array1<f64>, dt: f64) -> Array1<f64>
where
    F: Fn(f64, &Array1<f64>) -> Array1<f64>,
{
    let k1 = f(t, y);
    let k2 = f(t + 0.5 * dt, &(y + &(&k1 * (0.5 * dt))));
    let k3 = f(t + 0.5 * dt, &(y + &(&k2 * (0.5 * dt))));
    let k4 = f(t + dt, &(y + &(&k3 * dt)));

    y + &((&k1 + &(&k2 * 2.0) + &(&k3 * 2.0) + &k4) * (dt / 6.0))
}

/// Integrate using fixed-step RK4 from t_start to t_end.
/// Returns (times, states) where states is Vec of Array1<f64>.
pub fn rk4_integrate<F>(f: F, t_span: (f64, f64), y0: Array1<f64>, dt: f64) -> (Vec<f64>, Vec<Array1<f64>>)
where
    F: Fn(f64, &Array1<f64>) -> Array1<f64>,
{
    let (t_start, t_end) = t_span;
    let mut t = t_start;
    let mut y = y0;
    let mut times = vec![t];
    let mut states = vec![y.clone()];

    while t < t_end {
        let step = dt.min(t_end - t);
        if step <= 0.0 {
            break;
        }
        y = rk4_step(&f, t, &y, step);
        t += step;
        times.push(t);
        states.push(y.clone());
    }

    (times, states)
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_rk4_step_exponential() {
        // dy/dt = y  => solution y(t) = y0 * e^t
        let f = |_t: f64, y: &Array1<f64>| y.clone();
        let y0 = array![1.0];
        let y_next = rk4_step(f, 0.0, &y0, 0.1);
        // e^0.1 ≈ 1.105170918
        assert!((y_next[0] - 1.105170918).abs() < 1e-5);
    }

    #[test]
    fn test_rk4_integrate() {
        let f = |_t: f64, y: &Array1<f64>| y.clone();
        let y0 = array![1.0];
        let (times, states) = rk4_integrate(f, (0.0, 0.5), y0, 0.1);
        assert_eq!(times.len(), 6);
        let final_val = states.last().unwrap()[0];
        // e^0.5 ≈ 1.648721
        assert!((final_val - 1.648721).abs() < 0.02);
    }
}
