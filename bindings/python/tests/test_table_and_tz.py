# Copyright (c) 2026 Nathaniel T. Berry
# Licensed under the Apache License, Version 2.0.

from pathlib import Path

from symworx.core.io import load_numeric_table
from symworx.core.math import TimeZone, is_us_eastern_dst


def test_whitespace_explicit_names(tmp_path: Path):
    p = tmp_path / "rr.txt"
    p.write_text("0.000 1.304\n1.304 1.304\n", encoding="utf-8")
    t = load_numeric_table(str(p), delimiter="whitespace", has_headers=False, names=["t_s", "rr_s"])
    assert t.headers == ["t_s", "rr_s"]
    assert t.column("t_s")[0] == 0.0
    assert t.column("rr_s")[0] == 1.304


def test_headered_csv_skips_non_numeric(tmp_path: Path):
    p = tmp_path / "t.csv"
    p.write_text("x,y,label\n1.0,2.0,a\n3.0,4.0,b\n", encoding="utf-8")
    t = load_numeric_table(str(p))
    assert t.headers == ["x", "y"]
    assert t.n_rows() == 2


def test_us_eastern_august_is_edt():
    tz = TimeZone.parse("US/Eastern")
    assert tz.offset_hours(2017, 8, 14, 8, 20, 0) == -4
    assert is_us_eastern_dst(2017, 8, 14, 8, 20, 0)
    assert TimeZone.est().offset_hours(2017, 8, 14, 8, 20, 0) == -5
    unix = tz.local_to_unix(2017, 8, 14, 8, 20, 0)
    y, mo, d, h, mi, s = tz.unix_to_local(unix)
    assert (y, mo, d, h, mi, s) == (2017, 8, 14, 8, 20, 0)
