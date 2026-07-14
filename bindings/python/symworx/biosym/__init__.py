# Copyright (c) 2026 SymWorx. All rights reserved.
"""Re-export ``symworx._lib.biosym`` (gait types also available at top level)."""

from __future__ import annotations

from symworx import _lib

_rust = _lib.biosym
globals().update(
    {name: getattr(_rust, name) for name in dir(_rust) if not name.startswith("_")}
)

from . import biomechanics as biomechanics  # noqa: E402,F401
from . import cpg as cpg  # noqa: E402,F401
from . import physiology as physiology  # noqa: E402,F401
