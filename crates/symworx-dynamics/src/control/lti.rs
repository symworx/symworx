// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Discrete-time linear time-invariant (LTI) state-space models.
//!
//! ```text
//! x_{k+1} = A x_k + B u_k
//! y_k     = C x_k + D u_k
//! ```

use ndarray::{
    Array1,
    Array2,
};

/// Discrete-time LTI system.
#[derive(Debug, Clone)]
pub struct LtiDiscrete {
    /// State transition `A` (n × n).
    pub a: Array2<f64>,
    /// Input matrix `B` (n × m). `None` ⇒ autonomous system.
    pub b: Option<Array2<f64>>,
    /// Output matrix `C` (p × n).
    pub c: Array2<f64>,
    /// Feedthrough `D` (p × m). `None` ⇒ zero.
    pub d: Option<Array2<f64>>,
}

impl LtiDiscrete {
    /// Create a fully specified discrete LTI system.
    pub fn new(
        a: Array2<f64>,
        b: Option<Array2<f64>>,
        c: Array2<f64>,
        d: Option<Array2<f64>>,
    ) -> Self {
        let n = a.nrows();
        assert_eq!(a.ncols(), n, "A must be square");
        assert_eq!(c.ncols(), n, "C cols must equal state dim");
        if let Some(ref b) = b {
            assert_eq!(b.nrows(), n, "B rows must equal state dim");
            if let Some(ref d) = d {
                assert_eq!(d.nrows(), c.nrows(), "D rows must equal output dim");
                assert_eq!(d.ncols(), b.ncols(), "D cols must equal input dim");
            }
        }
        Self { a, b, c, d }
    }

    /// Autonomous system `x⁺ = A x`, `y = C x` (full-state output if `c = I`).
    pub fn autonomous(a: Array2<f64>) -> Self {
        let n = a.nrows();
        Self::new(a, None, Array2::eye(n), None)
    }

    /// State dimension `n`.
    pub fn n_states(&self) -> usize {
        self.a.nrows()
    }

    /// Input dimension `m` (0 if no `B`).
    pub fn n_inputs(&self) -> usize {
        self.b.as_ref().map(|b| b.ncols()).unwrap_or(0)
    }

    /// Output dimension `p`.
    pub fn n_outputs(&self) -> usize {
        self.c.nrows()
    }

    /// One step: returns `(x_next, y)`.
    pub fn step(&self, x: &Array1<f64>, u: Option<&Array1<f64>>) -> (Array1<f64>, Array1<f64>) {
        assert_eq!(x.len(), self.n_states());
        let mut x_next = self.a.dot(x);
        if let (Some(b), Some(u)) = (&self.b, u) {
            assert_eq!(u.len(), b.ncols());
            x_next = x_next + b.dot(u);
        }
        let mut y = self.c.dot(x);
        if let (Some(d), Some(u)) = (&self.d, u) {
            y = y + d.dot(u);
        }
        (x_next, y)
    }

    /// Simulate open-loop for `n_steps` from `x0`.
    ///
    /// `inputs[k]` is applied at step `k` (length `n_steps`). If `None`, free response.
    pub fn simulate(
        &self,
        x0: &Array1<f64>,
        n_steps: usize,
        inputs: Option<&[Array1<f64>]>,
    ) -> LtiSimResult {
        if let Some(us) = inputs {
            assert_eq!(us.len(), n_steps, "inputs length must equal n_steps");
        }
        let mut states = Vec::with_capacity(n_steps + 1);
        let mut outputs = Vec::with_capacity(n_steps);
        let mut x = x0.to_owned();
        states.push(x.clone());
        for k in 0..n_steps {
            let u = inputs.map(|us| &us[k]);
            let (x_next, y) = self.step(&x, u);
            outputs.push(y);
            x = x_next;
            states.push(x.clone());
        }
        LtiSimResult { states, outputs }
    }

    /// Closed-loop under static state feedback `u = −K x + r` (reference `r` optional).
    ///
    /// Returns a new LTI with `A_cl = A − B K`. Input of the closed-loop system
    /// is the reference `r` (same dimension as original `u`) when `B` is present.
    ///
    /// `k_gain` is `m × n` (rows = inputs, cols = states).
    pub fn state_feedback(&self, k_gain: &Array2<f64>) -> Self {
        let b = self
            .b
            .as_ref()
            .expect("state feedback requires an input matrix B");
        assert_eq!(k_gain.nrows(), b.ncols(), "K rows must equal number of inputs");
        assert_eq!(k_gain.ncols(), self.n_states(), "K cols must equal state dim");
        let a_cl = &self.a - &b.dot(k_gain);
        // r enters through B: x⁺ = A_cl x + B r
        Self::new(
            a_cl,
            Some(b.clone()),
            self.c.clone(),
            self.d.clone(),
        )
    }

    /// Discrete double-integrator (1D position/velocity) with sample time `dt`.
    ///
    /// State `[p, v]`, input acceleration `u`, output position.
    pub fn double_integrator(dt: f64) -> Self {
        assert!(dt > 0.0);
        let a = Array2::from_shape_vec(
            (2, 2),
            vec![1.0, dt, 0.0, 1.0],
        )
        .unwrap();
        let b = Array2::from_shape_vec((2, 1), vec![0.5 * dt * dt, dt]).unwrap();
        let c = Array2::from_shape_vec((1, 2), vec![1.0, 0.0]).unwrap();
        Self::new(a, Some(b), c, None)
    }

    /// Scalar discrete plant `x⁺ = a x + b u`, `y = x`.
    pub fn scalar(a: f64, b: f64) -> Self {
        Self::new(
            Array2::from_shape_vec((1, 1), vec![a]).unwrap(),
            Some(Array2::from_shape_vec((1, 1), vec![b]).unwrap()),
            Array2::eye(1),
            None,
        )
    }
}

/// Result of an LTI simulation.
#[derive(Debug, Clone)]
pub struct LtiSimResult {
    /// States `x_0 … x_{n}` (length `n_steps + 1`).
    pub states: Vec<Array1<f64>>,
    /// Outputs `y_0 … y_{n-1}` (length `n_steps`).
    pub outputs: Vec<Array1<f64>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_autonomous_decay() {
        let sys = LtiDiscrete::autonomous(array![[0.5, 0.0], [0.0, 0.8]]);
        let (x1, y) = sys.step(&array![2.0, 1.0], None);
        assert!((x1[0] - 1.0).abs() < 1e-15);
        assert!((x1[1] - 0.8).abs() < 1e-15);
        assert_eq!(y, array![2.0, 1.0]);
    }

    #[test]
    fn test_double_integrator_step() {
        let sys = LtiDiscrete::double_integrator(0.1);
        let (x1, y) = sys.step(&array![0.0, 0.0], Some(&array![1.0]));
        // p += 0.5 dt² u, v += dt u
        assert!((x1[0] - 0.005).abs() < 1e-12);
        assert!((x1[1] - 0.1).abs() < 1e-12);
        assert!((y[0]).abs() < 1e-15);
    }

    #[test]
    fn test_state_feedback_stabilizes() {
        // Unstable scalar plant x⁺ = 1.2 x + u
        let plant = LtiDiscrete::scalar(1.2, 1.0);
        // u = −0.5 x  ⇒ A_cl = 1.2 − 0.5 = 0.7
        let k = array![[0.5]];
        let cl = plant.state_feedback(&k);
        assert!((cl.a[[0, 0]] - 0.7).abs() < 1e-12);

        let sim = cl.simulate(&array![1.0], 20, None);
        let final_x = sim.states.last().unwrap()[0].abs();
        assert!(final_x < 0.01, "should decay, got {final_x}");
    }

    #[test]
    fn test_simulate_with_inputs() {
        let sys = LtiDiscrete::scalar(0.0, 1.0); // x⁺ = u
        let us: Vec<_> = (0..5).map(|i| array![i as f64]).collect();
        let sim = sys.simulate(&array![0.0], 5, Some(&us));
        assert_eq!(sim.states.len(), 6);
        assert!((sim.states[1][0] - 0.0).abs() < 1e-15);
        assert!((sim.states[2][0] - 1.0).abs() < 1e-15);
        assert!((sim.states[5][0] - 4.0).abs() < 1e-15);
    }
}
