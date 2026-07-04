#!/usr/bin/env python3
"""Turn an ffxiv_bossmod replay .log into a readable rotation timeline ("轴").

bossmod replay logs are pipe-delimited with a 3-letter mnemonic as the second
field. The vast majority of lines are noise for rotation analysis (FRAM frame
ticks, MOVE movement, IPCS raw packets). This script keeps the events that
matter for reading a rotation and prints them in order with relative timing:

  * CST!  an action actually fired (the rotation itself)
  * CST+  a hard cast started (annotated onto the matching CST! as its cast time)
  * CPET  active pet/demi changed (summon swaps)
  * DIE+  a death
  * STA+  a status was gained (only with --statuses; verbose)

It auto-detects the focal actor (the one that uses the most actions, i.e. the
player) unless you pass --actor. Works for any job/log, not just Summoner, and
handles both condensed-text and verbose-text replay exports.

Usage:
    python tools/parse_rotation.py <log> [--actor 10023A4D] [--statuses]
                                         [--targets] [--summary-only]
"""
from __future__ import annotations

import argparse
import re
import sys
from collections import Counter

# Make Chinese/Japanese names print correctly regardless of the OS console codepage.
try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

# Line types we care about; everything else (FRAM/MOVE/IPCS/...) is dropped.
ACTION = "CST!"
CAST_START = "CST+"
PET = "CPET"
DEATH_ON = "DIE+"
STATUS_ON = "STA+"
ACTOR_ADD = "ACT+"

_TIME_RE = re.compile(r"T(\d+):(\d+):([\d.]+)")
_ACTION_RE = re.compile(r"(\d+) '(.*)'")
_HEX_ID_RE = re.compile(r"^[0-9A-Fa-f]+$")


def parse_seconds(timestamp: str) -> float | None:
    """`2026-06-16T06:07:13.6645767+08:00` -> seconds-of-day as a float.

    Parsed by hand to avoid datetime's fussiness about 7-digit fractions / tz.
    """
    m = _TIME_RE.search(timestamp)
    if not m:
        return None
    return int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))


def parse_action(field: str) -> tuple[int, str]:
    """`Spell 16508 '能量吸收'` -> (16508, '能量吸收')."""
    m = _ACTION_RE.search(field)
    return (int(m.group(1)), m.group(2)) if m else (0, field.strip())


def normalize_actor_ref(field: str) -> str:
    """Normalize actor references across replay formats.

    Old logs use bare ids like `10023A4D`; newer verbose logs embed richer actor
    descriptors like `100241B1/0/辉夜姬/Player/...`. In both cases we key actors
    by their object id so casts/statuses/pet swaps can be correlated.
    """
    raw = field.strip()
    head = raw.split("/", 1)[0].strip()
    return head.upper() if _HEX_ID_RE.fullmatch(head) else raw.upper()


def embedded_actor_name(field: str) -> str | None:
    """Extract display name from composite actor refs like `id/.../name/type/...`."""
    parts = field.strip().split("/")
    if len(parts) >= 3 and parts[2]:
        return parts[2]
    return None


def main() -> None:
    ap = argparse.ArgumentParser(description="Parse a bossmod replay log into a rotation timeline.")
    ap.add_argument("log", help="path to the .log file")
    ap.add_argument("--actor", help="focal actor id (hex, e.g. 10023A4D); default: busiest caster")
    ap.add_argument("--statuses", action="store_true", help="also show statuses the focal actor gains")
    ap.add_argument("--targets", action="store_true", help="show each action's target")
    ap.add_argument("--deaths", action="store_true", help="also show actor deaths")
    ap.add_argument("--summary-only", action="store_true", help="skip the timeline, only print the summary")
    args = ap.parse_args()

    with open(args.log, encoding="utf-8", errors="replace") as fh:
        rows = [ln.rstrip("\n").split("|") for ln in fh if "|" in ln]

    # actor id -> display name, from ACT+ spawns. Layout: ACT+|id|.|.|.|name|...
    names: dict[str, str] = {}
    for r in rows:
        if r[1] == ACTOR_ADD and len(r) > 6:
            names[normalize_actor_ref(r[2])] = r[6]

    def name_of(actor_ref: str) -> str:
        normalized = normalize_actor_ref(actor_ref)
        return names.get(normalized) or embedded_actor_name(actor_ref) or actor_ref

    # Focal actor = the one that fires the most actions, unless overridden.
    casters = Counter(normalize_actor_ref(r[2]) for r in rows if r[1] == ACTION and len(r) > 2)
    if not casters and not args.actor:
        print("No CST! (action) lines found — is this a bossmod replay log?")
        return
    focal = normalize_actor_ref(args.actor or casters.most_common(1)[0][0])

    # Match hard casts (CST+) to the action that follows: actor+action -> last cast total time.
    pending_cast: dict[tuple[str, int], float] = {}

    events: list[tuple[float, str, str]] = []  # (seconds, kind, text)
    action_seq: list[tuple[float, int, str]] = []  # (seconds, id, name) for the focal actor

    for r in rows:
        kind = r[1]
        t = parse_seconds(r[0])
        if t is None:
            continue

        actor = normalize_actor_ref(r[2]) if len(r) > 2 else ""

        if kind == CAST_START and len(r) > 6 and actor == focal:
            aid, _ = parse_action(r[3])
            total = r[6].split("/")[-1]  # "elapsed/total"
            try:
                pending_cast[(focal, aid)] = float(total)
            except ValueError:
                pass

        elif kind == ACTION and len(r) > 4 and actor == focal:
            aid, aname = parse_action(r[3])
            note = ""
            cast = pending_cast.pop((focal, aid), None)
            if cast and cast > 0.2:
                note = f"  [hard cast {cast:.2f}s]"
            if args.targets and len(r) > 4:
                note += f"  -> {name_of(r[4])}"
            events.append((t, "act", f"{aname} ({aid}){note}"))
            action_seq.append((t, aid, aname))

        elif kind == PET and len(r) > 2:
            pid = r[2]
            # E0000000 = dismissed; skip it (the next summon line already implies the swap).
            if not pid.startswith("E000"):
                events.append((t, "pet", f">> summon: {name_of(pid)}"))

        elif kind == DEATH_ON and args.deaths and len(r) > 2:
            events.append((t, "die", f"** {name_of(r[2])} dies"))

        elif kind == STATUS_ON and args.statuses and len(r) > 5 and actor == focal:
            sid, sname = parse_action(r[4])
            dur = r[6] if len(r) > 6 else "?"
            events.append((t, "sta", f"+buff {sname} ({sid}, {dur}s)"))

    if not action_seq:
        print(f"No actions found for actor {focal}.")
        return

    start = action_seq[0][0]
    end = action_seq[-1][0]

    # ---- summary ----
    print(f"file      : {args.log}")
    print(f"focal     : {focal} '{name_of(focal)}'")
    print(f"duration  : {end - start:.1f}s   actions: {len(action_seq)}")
    print()
    print("action counts (id  ×count  name):")
    counts = Counter((aid, aname) for _, aid, aname in action_seq)
    for (aid, aname), c in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"  {aid:<7} ×{c:<3} {aname}")

    if args.summary_only:
        return

    # ---- timeline ----
    print()
    print(f"{'t(s)':>7} {'Δ':>6}  event")
    print(f"{'-' * 7} {'-' * 6}  {'-' * 44}")
    prev_act_t: float | None = None
    for t, kind, text in events:
        rel = t - start
        if kind == "act":
            delta = "" if prev_act_t is None else f"{t - prev_act_t:.2f}"
            prev_act_t = t
            print(f"{rel:7.2f} {delta:>6}  {text}")
        else:
            # annotations (pet/death/buff) indent under the timeline
            print(f"{rel:7.2f} {'':>6}    {text}")


if __name__ == "__main__":
    main()
