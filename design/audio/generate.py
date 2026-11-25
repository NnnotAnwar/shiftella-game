#!/usr/bin/env python3
"""Music and SFX generator for Shiftella, using the ElevenLabs API.

The generated audio is NOT committed to the public repository (the ElevenLabs
licence allows use inside the game, not redistribution). Use your own key to make
your own local copies; the game also runs without any audio.

Run (from the repository root):
    python3 -I design/audio/generate.py                 # everything not generated yet
    python3 -I design/audio/generate.py --only rush_loop,ui_click
    python3 -I design/audio/generate.py --kind sfx --force   # regenerate all SFX
    python3 -I design/audio/generate.py --dry-run        # show the plan, no requests
    python3 -I design/audio/generate.py --reconvert      # only rebuild OGG/WAV from raw/

Key: the ELEVENLABS_API_KEY environment variable, or a line
ELEVENLABS_API_KEY=... in .env at the repository root (git-ignored).
The key is never printed and never written to the logs.

Endpoints (checked against the docs on 2026-11-25):
    POST /v1/music            body: prompt, music_length_ms (3000..600000), force_instrumental, model_id
    POST /v1/sound-generation body: text, duration_seconds (0.5..30), loop, prompt_influence (0..1), model_id
    output_format is a query parameter; we use mp3_44100_128 (available on every plan),
    then ffmpeg converts to OGG Vorbis / PCM WAV for macroquad.
Python standard library only, no dependencies.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

API = "https://api.elevenlabs.io"
RAW_FORMAT = "mp3_44100_128"

HERE = Path(__file__).resolve().parent          # design/audio
ROOT = HERE.parents[1]                          # repository root
PROMPTS = HERE / "prompts.json"
RAW_DIR = HERE / "raw"
LOG_JSONL = HERE / "generation-log.jsonl"

# Loudness targets (see sound-design.md, section "Loudness and mix").
LOUDNESS = {
    "music": {"I": -18.0, "TP": -1.5, "LRA": 11.0},
    "ambience": {"I": -24.0, "TP": -2.0, "LRA": 7.0},
}

# Response headers that are safe to keep in the log (they never contain the key).
SAFE_HEADERS = ("request-id", "x-request-id", "song-id", "character-cost",
                "x-character-count", "history-item-id", "content-type")


# ---------------------------------------------------------------- key
def load_key() -> str | None:
    key = os.environ.get("ELEVENLABS_API_KEY", "").strip()
    if key:
        return key
    env = ROOT / ".env"
    if env.is_file():
        for line in env.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("export "):
                line = line[len("export "):].strip()
            if line.startswith("ELEVENLABS_API_KEY="):
                val = line.split("=", 1)[1].strip().strip('"').strip("'")
                if val:
                    return val
    return None


# ---------------------------------------------------------------- HTTP
def _request(method: str, path: str, key: str, body: dict | None = None,
             query: str = "", timeout: int = 600):
    url = f"{API}{path}" + (f"?{query}" if query else "")
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("xi-api-key", key)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        headers = {k.lower(): v for k, v in resp.headers.items()}
        return resp.read(), headers


def credits_used(key: str) -> int | None:
    """Credits used in the current period (character_count), or None."""
    try:
        raw, _ = _request("GET", "/v1/user/subscription", key, timeout=30)
        return int(json.loads(raw).get("character_count"))
    except Exception:
        return None  # the key may lack the user_read permission; that's fine


def subscription_summary(key: str) -> dict | None:
    try:
        raw, _ = _request("GET", "/v1/user/subscription", key, timeout=30)
        d = json.loads(raw)
        return {k: d.get(k) for k in ("tier", "character_count", "character_limit",
                                      "next_character_count_reset_unix", "status")}
    except Exception:
        return None


# ---------------------------------------------------------------- ffmpeg
def have(tool: str) -> bool:
    return shutil.which(tool) is not None


def probe_duration(path: Path) -> float | None:
    if not have("ffprobe"):
        return None
    try:
        out = subprocess.run(
            ["ffprobe", "-v", "error", "-show_entries", "format=duration",
             "-of", "default=nw=1:nk=1", str(path)],
            capture_output=True, text=True, check=True).stdout.strip()
        return round(float(out), 2)
    except Exception:
        return None


def vorbis_args() -> list[str]:
    enc = subprocess.run(["ffmpeg", "-hide_banner", "-encoders"],
                         capture_output=True, text=True).stdout
    if "libvorbis" in enc:
        return ["-c:a", "libvorbis", "-q:a", "6"]
    # ffmpeg's built-in encoder: experimental, stereo only
    return ["-c:a", "vorbis", "-strict", "-2", "-ac", "2", "-q:a", "6"]


def loudnorm_filter(src: Path, target: dict) -> str:
    """Two-pass linear normalisation (keeps loop seams intact)."""
    base = f"loudnorm=I={target['I']}:TP={target['TP']}:LRA={target['LRA']}"
    res = subprocess.run(
        ["ffmpeg", "-hide_banner", "-nostats", "-i", str(src),
         "-af", base + ":print_format=json", "-f", "null", "-"],
        capture_output=True, text=True)
    txt = res.stderr
    try:
        m = json.loads(txt[txt.rindex("{"):txt.rindex("}") + 1])
        return (base + f":measured_I={m['input_i']}:measured_TP={m['input_tp']}"
                f":measured_LRA={m['input_lra']}:measured_thresh={m['input_thresh']}"
                f":offset={m['target_offset']}:linear=true")
    except Exception:
        return base


def convert(raw: Path, out: Path, normalize: str) -> Path:
    out.parent.mkdir(parents=True, exist_ok=True)
    if not have("ffmpeg"):
        dst = out.with_suffix(raw.suffix)
        shutil.copyfile(raw, dst)
        print(f"   ! ffmpeg not found — kept the original {dst.suffix}; convert later with --reconvert")
        return dst
    cmd = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(raw)]
    if normalize in LOUDNESS:
        cmd += ["-af", loudnorm_filter(raw, LOUDNESS[normalize])]
    cmd += ["-ar", "44100"]
    if out.suffix == ".ogg":
        cmd += vorbis_args()
    elif out.suffix == ".wav":
        cmd += ["-c:a", "pcm_s16le"]
    cmd.append(str(out))
    subprocess.run(cmd, check=True)
    return out


# ---------------------------------------------------------------- generation
def build_request(kind: str, item: dict) -> tuple[str, dict]:
    if kind == "music":
        body = {"prompt": item["prompt"],
                "music_length_ms": int(item["length_ms"]),
                "force_instrumental": bool(item.get("force_instrumental", True))}
        if item.get("model_id"):
            body["model_id"] = item["model_id"]
        return "/v1/music", body
    body = {"text": item["text"],
            "loop": bool(item.get("loop", False)),
            "prompt_influence": float(item.get("prompt_influence", 0.3))}
    if item.get("duration_seconds") is not None:
        body["duration_seconds"] = float(item["duration_seconds"])
    if item.get("model_id"):
        body["model_id"] = item["model_id"]
    return "/v1/sound-generation", body


def log(entry: dict) -> None:
    with LOG_JSONL.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")


def main() -> int:
    ap = argparse.ArgumentParser(description="ElevenLabs → assets/audio")
    ap.add_argument("--only", help="comma-separated ids")
    ap.add_argument("--kind", choices=["music", "sfx"])
    ap.add_argument("--force", action="store_true", help="overwrite existing files")
    ap.add_argument("--dry-run", action="store_true", help="no API requests")
    ap.add_argument("--no-convert", action="store_true", help="raw mp3 only")
    ap.add_argument("--reconvert", action="store_true", help="rebuild from raw/ without the API")
    args = ap.parse_args()

    spec = json.loads(PROMPTS.read_text(encoding="utf-8"))
    only = {s.strip() for s in args.only.split(",")} if args.only else None
    jobs = [(kind, it) for kind in ("music", "sfx") for it in spec.get(kind, [])
            if (not args.kind or args.kind == kind) and (not only or it["id"] in only)]
    if only:
        missing = only - {it["id"] for _, it in jobs}
        if missing:
            print(f"Unknown ids: {', '.join(sorted(missing))}")
            return 2

    RAW_DIR.mkdir(parents=True, exist_ok=True)

    if args.reconvert:
        for kind, it in jobs:
            raw = RAW_DIR / f"{it['id']}.mp3"
            if raw.exists():
                out = convert(raw, ROOT / it["out"], it.get("normalize", "none"))
                print(f"✓ {it['id']}: {out.relative_to(ROOT)} ({probe_duration(out)} s)")
        return 0

    key = None if args.dry_run else load_key()
    if not args.dry_run and not key:
        print("ELEVENLABS_API_KEY not found (neither in the environment nor in .env at the repo root).\n"
              "Add a line ELEVENLABS_API_KEY=... with your own key to .env and run again.\n"
              ".env is git-ignored — never commit it.")
        return 1

    if key:
        summary = subscription_summary(key)
        if summary:
            print(f"Plan: {summary.get('tier')}, used "
                  f"{summary.get('character_count')} / {summary.get('character_limit')} credits")

    total = 0
    for kind, it in jobs:
        out = ROOT / it["out"]
        raw = RAW_DIR / f"{it['id']}.mp3"
        if out.exists() and not args.force:
            print(f"· {it['id']}: {it['out']} already exists — skipped (--force to overwrite)")
            continue
        path, body = build_request(kind, it)
        if args.dry_run:
            print(f"[dry-run] POST {path}?output_format={RAW_FORMAT} → {it['out']}\n"
                  f"          {json.dumps({k: v for k, v in body.items() if k not in ('prompt', 'text')}, ensure_ascii=False)}")
            continue
        print(f"→ {it['id']} ({kind}) …", flush=True)
        before = credits_used(key)
        try:
            audio, headers = _request("POST", path, key, body,
                                      query=f"output_format={RAW_FORMAT}")
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", "replace")[:600]
            print(f"   ✗ HTTP {e.code}: {detail}")
            log({"id": it["id"], "kind": kind, "ok": False, "http": e.code,
                 "detail": detail, "at": dt.datetime.now().isoformat(timespec="seconds")})
            continue
        except urllib.error.URLError as e:
            print(f"   ✗ network: {e.reason}")
            continue
        raw.write_bytes(audio)
        after = credits_used(key)
        cost = (after - before) if (before is not None and after is not None) else None
        if cost is not None:
            total += cost
        final = raw if args.no_convert else convert(raw, out, it.get("normalize", "none"))
        dur = probe_duration(final)
        entry = {"id": it["id"], "kind": kind, "ok": True, "out": str(final.relative_to(ROOT)),
                 "raw": str(raw.relative_to(ROOT)), "raw_bytes": len(audio),
                 "duration_s": dur, "credits": cost, "endpoint": path,
                 "params": {k: v for k, v in body.items() if k not in ("prompt", "text")},
                 "prompt": body.get("prompt") or body.get("text"),
                 "headers": {h: headers[h] for h in SAFE_HEADERS if h in headers},
                 "at": dt.datetime.now().isoformat(timespec="seconds")}
        log(entry)
        print(f"   ✓ {entry['out']}  {dur} s  credits: {cost if cost is not None else 'n/a'}")

    if key and not args.dry_run:
        print(f"Credits used in this run: {total}")
        summary = subscription_summary(key)
        if summary:
            print(f"Usage now: {summary.get('character_count')} / {summary.get('character_limit')}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
