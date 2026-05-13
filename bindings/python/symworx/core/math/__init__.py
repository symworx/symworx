# core/python/src/symworx/core/math/__init__.py
# Copyright (C) 2026 cSYMd, All rights reserved.

from ..core import math as _rust_io

# Re-export 
globals().update({
    name: getattr(_rust_math, name)
    for name in dir(_rust_math)
    if not name.startswith("_")
})
