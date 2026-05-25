# core/python/src/symworx/core/math/__init__.py
# Copyright (c) 2026 SymWorx. All rights reserved.

from ..core import math as _rust_io

# Re-export 
globals().update({
    name: getattr(_rust_math, name)
    for name in dir(_rust_math)
    if not name.startswith("_")
})
