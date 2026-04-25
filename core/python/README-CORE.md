# SymWorx-Core

**SymWorx-Core** provides the foundational computational utilities shared across the broader symworx namespace.
It is a central component of the **Computational Systems for Modeling Dynamics** (cSYM‑d) ecosystem, supporting modeling, simulation, signal processing, and data‑centric workflows across Python, web, embedded, and edge environments.

## Getting Started
1. [Setting up Rust](#rust-setup)
2. [Setting up Python](#python-setup)

## Submodules
1. [Backend](#backend)
2. [Dynamics](#dynamics)
3. [Filters](#filters)
   i. [Adaptive](#adaptive)
   ii. [Linear](#linear)
   iii. [Nonlinear](#nonlinear)
4. [IO](#io)
5. [Processing](#processing)
6. [Statistics](#statistics)




## Getting Started
### Rust Setup

### Python Setup
The **SymWorx** ecosystem is written in [**Rust**](https://www.rust-lang.org) and uses the **pyo3** crate to create [**Python**](https://www.python.org) wrappers. **pyo3** currently only supports up to python 3.13 at the time of development. 
**Python 3.12** has been used for development. 

As a result, you will need to install **Python 3.12** to leverage the Python packages.



## Submodules
### Backend
### Dynamics
### Filters 
#### Adaptive
#### Linear

``` python
# Example: Bandpass Filter
from symworx.core.filters import BandpassFilter

f = BandpassFilter(fs, low, high, q)
y = f.process(x)

```

#### Nonlinear

``` python
# Example: Kalman Filter
from symworx.core.filters import KalmanFilter

kf = KalmanFilter(dt=1.0, process_var=1e-5, meas_var=0.1)

for z in measurements:
    kf.predict()
    kf.update(z)
    pos, vel = kf.state()
```

#### Time Frequency

### IO
### Processing
### Statistics
