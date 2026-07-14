# Copyright (c) 2026 SymWorx. All rights reserved.
"""Re-export ``symworx._lib.loadsym.nutrition``."""

from __future__ import annotations

from symworx import _lib

_rust = _lib.loadsym.nutrition
globals().update(
    {name: getattr(_rust, name) for name in dir(_rust) if not name.startswith("_")}
)
