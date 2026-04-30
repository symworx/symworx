# SymWorx

**SymWorx** is a multi‑use computational framework designed for broad applicability across embedded systems, scientific computing, web applications, and educational environments.
At its core, **SymWorx** provides a **Rust kernel** with **Python bindings**, ensuring consistency, safety, and performance across platforms.

**SymWorx** is part of the **Computational Systems for Modeling & Dynamics** (cSYM-d) ecosystem.

## Philosophy

The cSYM‑d philosophy emphasizes:
i) **security** by minimizing unsafe code, reducing unintended execution paths, and lowering supply‑chain risk,
ii) **robustness** through predictable behavior, strong typing, and explicit error handling, and 
iii) **scalability** via consistent APIs across embedded, desktop, and cloud environments.

Much of the original work was developed in Python, but is now being rewritten in Rust.
This shift was motivated by both a desire to deeply learn Rust and the opportunity to build a portable, high‑assurance computational engine that can 
i) run on microcontrollers and bare‑metal systems, 
ii) integrate seamlessly with Python for education, data science, and rapid prototyping, 
iii) serve as a stable foundation for higher‑level simulation frameworks

**License:**
This project is licensed under the [Mozilla Public License, version 2.0](https://www.mozilla.org/media/MPL/2.0/index.f75d2927d3c1.txt).

**Versioning Issues:**
If you encounter Python versioning issues, you can set environment variables to specify the Python version:
```
export PYO3_PYTHON=python3.12
export PYTHON_SYS_EXECUTABLE=python3.12
```

## Repository Structure

This **SymWorx** repository is a monorepo that contains a variety of `Rust` crates and `Python` packages; additional details can be found below.


### [Core](core/README-CORE.md)

**Overview**: The `core` crate contains a variety of resources used across the subsequent simulation focused crates.
This includes backend resources, io, filters, processing, nonlinear dynamics, and statistics.


### BioSym

**Overview**: The `biosym` crate contains modeling and simulation tools for biological signals and responses. 
Specifically, `biosym` contains physiological (ppg and respiratory) and biomechanical (gait). 
It also contains a central pattern generator (cpg) that integrates these signals.


### LoadSym

**Overview**: The `loadsym` crate contains resources for quantifying and optimizing (exercise programming) training load (physiological and mechanical).
It also contains resources centered around nutrition and energy 
(e.g., basal metabolic rate, total daily energy expenditure, etc.)


### RunSym

**Overview**: The `runsym` crate contains resources and for modeling the physiological and biomechanical responses to running. 
These resources are focused on athletic performance but may later be expanded to isolate product related effects on performance 
(e.g., the role of footwear or apparel on overall run performance or running related injury risk).
