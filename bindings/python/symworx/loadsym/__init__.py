# Copyright (c) 2026 SymWorx. All rights reserved.
"""Re-export ``symworx._lib.loadsym``."""

from __future__ import annotations

from symworx import _lib

_rust = _lib.loadsym
globals().update(
    {name: getattr(_rust, name) for name in dir(_rust) if not name.startswith("_")}
)

from . import load as load  # noqa: E402,F401
from . import nutrition as nutrition  # noqa: E402,F401
