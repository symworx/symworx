# core/python/src/symworx/core/statistics/__init__.py
# Copyright (C) 2026 cSYMd, All rights reserved.

from ..core import statistics as _rust_statistics

# Re-export 
globals().update({
    name: getattr(_rust_statistics, name)
    for name in dir(_rust_statistics)
    if not name.startswith("_")
})
