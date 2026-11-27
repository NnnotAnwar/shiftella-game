#!/usr/bin/env python3
"""Summarise a SHIFTELLA_PROFILE=path.csv frame profile.

    python3 tools/profile_summary.py run.csv [more.csv ...] [--skip N] [--by-scene]

Prints, per CSV (and per on-screen label with --by-scene), the median / p95 / p99 / max of
each timed part (main-thread CPU ms; frame_wall and present_wall are wall ms) and the totals
of the counters. `--skip N` drops the first N frames (warm-up: atlas fills, face bakes).
Several CSVs of the same scenario are pooled (repeat runs on a busy machine).
"""

import csv
import sys

TIMES = [
    "frame",
    "faces",
    "sim",
    "hands",
    "refresh",
    "world",
    "lighting",
    "overlay",
    "ui",
    "photos",
    "walk",
    "present",
    "frame_wall",
    "present_wall",
]
COUNTS = [
    "glyphs",
    "glyph_flushes",
    "statics",
    "render_targets",
    "face_bakes",
    "photo_renders",
    "ticks",
    "draw_calls",
    "buffer_updates",
    "upload_kb",
    "vertices",
    "ui_verts",
    "brush_verts",
]


def pct(xs, p):
    if not xs:
        return 0.0
    xs = sorted(xs)
    k = (len(xs) - 1) * p / 100.0
    lo = int(k)
    hi = min(lo + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)


def summarise(title, rows):
    print(f"\n== {title}: {len(rows)} frames")
    print(f"{'part':>13} {'median':>8} {'p95':>8} {'p99':>8} {'max':>8}")
    for t in TIMES:
        xs = [float(r[t]) for r in rows if t in r]
        if not xs or max(xs) == 0.0:
            continue
        print(
            f"{t:>13} {pct(xs, 50):8.2f} {pct(xs, 95):8.2f} {pct(xs, 99):8.2f} {max(xs):8.2f}"
        )
    tot = {c: sum(int(r[c]) for r in rows if c in r) for c in COUNTS}
    print("   counters: " + ", ".join(f"{c}={v}" for c, v in tot.items()))
    per = {c: pct([float(r[c]) for r in rows if c in r], 50) for c in ("draw_calls", "upload_kb", "vertices", "ui_verts", "brush_verts")}
    print("   per frame (median): " + ", ".join(f"{c}={v:.0f}" for c, v in per.items()))
    late = [r for r in rows[len(rows) // 2 :]]
    late_tot = {c: sum(int(r[c]) for r in late if c in r) for c in COUNTS if c != "ticks"}
    print("   2nd half: " + ", ".join(f"{c}={v}" for c, v in late_tot.items()))


def main():
    args = sys.argv[1:]
    skip = 0
    by_scene = False
    files = []
    i = 0
    while i < len(args):
        if args[i] == "--skip":
            skip = int(args[i + 1])
            i += 2
            continue
        if args[i] == "--by-scene":
            by_scene = True
        else:
            files.append(args[i])
        i += 1
    rows = []
    for f in files:
        with open(f, newline="") as fh:
            rows.extend(list(csv.DictReader(fh))[skip:])
    summarise(", ".join(files), rows)
    if by_scene:
        scenes = {}
        for r in rows:
            scenes.setdefault(r["scene"], []).append(r)
        for s, rs in sorted(scenes.items(), key=lambda kv: -len(kv[1])):
            summarise(s, rs)


if __name__ == "__main__":
    main()
