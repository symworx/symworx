# Copyright (c) 2026 PalEm Dynamics LLC
# Licensed under the Apache License, Version 2.0.

"""Re-export ``symworx._lib.loadsym.nutrition``."""

from __future__ import annotations

from symworx import _lib

_rust = _lib.loadsym.nutrition
globals().update(
    {name: getattr(_rust, name) for name in dir(_rust) if not name.startswith("_")}
)
