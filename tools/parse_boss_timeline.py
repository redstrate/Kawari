#!/usr/bin/env python3
"""Turn an ffxiv_bossmod replay .log into a readable BOSS + HELPER behaviour timeline.

Where `parse_rotation.py` follows the *player's* rotation, this follows what the
**enemies** do — the boss and its helper/clone actors — so you can read a duty's
mechanic flow and reproduce it server-side. It prints, in combat-relative time:

  * spawn / despawn       (ACT+ / ACT-)  — with OID (= DataId = BNpcBase), HP, pos
  * cast start            (CST+)         — spell, target, **cast Location**, cast time
  * cast fire             (CST!)         — spell, main target, **TargetPos**
  * targetable on/off     (ATG+ / ATG-)  — e.g. a boss going untargetable mid-jump
  * death                 (DIE+)
  * (opt) movement, statuses applied to players, action-effect damage

The boss is auto-detected as the non-ally enemy with the largest MaxHP. Every
other non-ally actor with an OID (helpers like 0x1B2, adds, nails, clones) is
shown too. By default the timeline is clipped to [boss pull .. boss HP hits 0].

Why this matters for Kawari: the verbose actor ref is
`Instance/OID/Name/Type/x/y/z/rot`, so `400049F2/D1/伊弗利特/Enemy` tells you the
boss is BNpcBase 0xD1 (209), and helper casts of an AoE action carry the ground
`Location` where the omen is drawn — exactly the data the director needs.

Usage:
    python tools/parse_boss_timeline.py <log>
        [--actor 400049F2] [--oid D1] [--moves] [--statuses] [--effects]
        [--targets] [--all] [--summary-only]
"""
from __future__ import annotations

import argparse
import re
import sys
from collections import Counter, defaultdict

# Make CJK names print correctly regardless of the OS console codepage.
try:
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

_TIME_RE = re.compile(r"T(\d+):(\d+):([\d.]+)")
_ACTION_RE = re.compile(r"(?:Spell|Ability|Item|\w+)?\s*(\d+)\s*'(.*)'")
_HEX_ID_RE = re.compile(r"^[0-9A-Fa-f]+$")


def parse_seconds(timestamp: str) -> float | None:
    """`2026-06-16T06:07:13.6645767+08:00` -> seconds-of-day as a float."""
    m = _TIME_RE.search(timestamp)
    if not m:
        return None
    return int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))


def parse_action(field: str) -> tuple[int, str]:
    """`Spell 1355 '地火喷发'` -> (1355, '地火喷发')."""
    m = _ACTION_RE.search(field)
    return (int(m.group(1)), m.group(2)) if m else (0, field.strip())


class Ref:
    """A parsed actor reference, `Instance/OID/Name/Type/x/y/z/rot` or a bare id."""

    __slots__ = ("id", "oid", "name", "type", "pos", "rot")

    def __init__(self, field: str):
        parts = field.strip().split("/")
        self.id = parts[0].upper() if parts and _HEX_ID_RE.fullmatch(parts[0]) else parts[0]
        if len(parts) >= 4:
            self.oid = parts[1].upper()
            self.name = parts[2]
            self.type = parts[3]
            self.pos = tuple(parts[4:7]) if len(parts) >= 7 else None
            self.rot = parts[7] if len(parts) >= 8 else None
        else:
            self.oid = self.name = self.type = None
            self.pos = self.rot = None

    def is_valid(self) -> bool:
        return bool(self.id) and self.id not in ("E0000000", "00000000", "FFFFFFFF")


def pos_str(pos) -> str:
    """('1.234','0.000','-5.6') -> '(1.2, -5.6)' (X,Z — the gameplay plane)."""
    if not pos:
        return "?"
    try:
        return f"({float(pos[0]):.1f}, {float(pos[2]):.1f})"
    except (ValueError, IndexError):
        return "/".join(pos)


def short(inst_id: str) -> str:
    """Last 4 hex of an instance id, for compact per-line tags."""
    return inst_id[-4:] if len(inst_id) > 4 else inst_id


class Actor:
    __slots__ = ("id", "oid", "name", "type", "max_hp", "is_ally", "owner")

    def __init__(self, inst_id: str):
        self.id = inst_id
        self.oid = "0"
        self.name = inst_id
        self.type = "?"
        self.max_hp = 0
        self.is_ally = False
        self.owner = None

    @property
    def label(self) -> str:
        return f"{short(self.id)} {self.name}({self.oid})"


def main() -> None:
    ap = argparse.ArgumentParser(description="Parse a bossmod replay log into a boss/helper behaviour timeline.")
    ap.add_argument("log", help="path to the .log file")
    ap.add_argument("--actor", help="focus a single actor instance id (hex, e.g. 400049F2)")
    ap.add_argument("--oid", help="focus actors by OID / DataId (hex, e.g. D1 for the boss, 1B2 for helpers)")
    ap.add_argument("--moves", action="store_true", help="include MOVE lines (downsampled to >2u steps)")
    ap.add_argument("--statuses", action="store_true", help="include statuses the enemies apply to players")
    ap.add_argument("--effects", action="store_true", help="include AIE+ action-effect (damage) lines")
    ap.add_argument("--targets", action="store_true", help="show each CST! fire's target list")
    ap.add_argument("--all", action="store_true", help="don't clip to the [pull .. boss death] window")
    ap.add_argument("--summary-only", action="store_true", help="skip the timeline, only per-actor cast counts")
    args = ap.parse_args()

    with open(args.log, encoding="utf-8", errors="replace") as fh:
        rows = [ln.rstrip("\n").split("|") for ln in fh if "|" in ln]

    # ---- pass 1: build the actor table from ACT+ (and fill gaps from refs) ----
    actors: dict[str, Actor] = {}

    def actor_for(inst_id: str) -> Actor:
        a = actors.get(inst_id)
        if a is None:
            a = actors[inst_id] = Actor(inst_id)
        return a

    def note_ref(ref: Ref) -> None:
        """Backfill name/oid/type for an actor seen only via a verbose ref."""
        if not ref.is_valid() or ref.oid is None:
            return
        a = actor_for(ref.id)
        if a.name == a.id and ref.name:
            a.name = ref.name
        if a.oid == "0":
            a.oid = ref.oid
        if a.type == "?":
            a.type = ref.type

    for r in rows:
        if len(r) < 3:
            continue
        kind = r[1].strip()
        if kind == "ACT+" and len(r) > 20:
            a = actor_for(r[2].upper())
            a.oid = r[3].upper()
            a.name = r[6] or a.name
            a.type = r[8]
            try:
                a.max_hp = max(a.max_hp, int(r[15]))
            except ValueError:
                pass
            a.is_ally = (r[20].strip().lower() == "true")
            owner = Ref(r[21])
            a.owner = owner.id if owner.is_valid() else None
        else:
            # backfill from any verbose ref fields on this row
            for f in r[2:]:
                if "/" in f and _HEX_ID_RE.fullmatch(f.split("/")[0]):
                    note_ref(Ref(f))

    if not actors:
        print("No actors found — is this a TextVerbose bossmod replay log?")
        return

    # ---- classify: focal = non-ally actors with a real OID (boss, helpers, adds) ----
    def is_focal(a: Actor) -> bool:
        return (not a.is_ally) and a.oid not in ("0", "")

    focal_ids = {aid for aid, a in actors.items() if is_focal(a)}
    if not focal_ids:
        print("No enemy/helper actors found (all actors are allies/players).")
        return

    # Boss = the focal actor that *fires the most actions* (the real boss runs the
    # rotation), tie-broken by MaxHP. MaxHP alone is ambiguous: retail pre-spawns the
    # Crimson Cyclone clones as extra full-HP boss-OID actors on the edge ring, so the
    # busiest caster is what actually distinguishes the live boss from its clones.
    fire_count: Counter = Counter()
    for r in rows:
        if len(r) > 2 and r[1].strip() == "CST!":
            cid = Ref(r[2]).id
            if cid in focal_ids:
                fire_count[cid] += 1
    boss = max(
        (actors[a] for a in focal_ids),
        key=lambda a: (fire_count[a.id], a.max_hp),
    )

    # apply --actor / --oid focus
    if args.actor:
        want = args.actor.upper()
        focal_ids = {aid for aid in focal_ids if aid == want or short(aid) == short(want)}
    if args.oid:
        want = args.oid.upper()
        focal_ids = {aid for aid in focal_ids if actors[aid].oid == want}
    if not focal_ids:
        print("No actors matched the --actor/--oid filter.")
        return

    # ---- determine combat window [start .. boss HP 0 / death] ----
    boss_pull_t: float | None = None
    boss_end_t: float | None = None
    for r in rows:
        if len(r) < 3:
            continue
        kind = r[1].strip()
        t = parse_seconds(r[0])
        if t is None:
            continue
        subj = Ref(r[2])
        if subj.id != boss.id:
            continue
        if kind in ("CST+", "CST!", "COM+") and boss_pull_t is None:
            boss_pull_t = t
        if kind == "HP" and len(r) > 3:
            try:
                if int(r[3]) <= 0:
                    boss_end_t = boss_end_t or t
            except ValueError:
                pass
        if kind == "DIE+":
            boss_end_t = boss_end_t or t

    clip = not args.all and boss_pull_t is not None
    lo = boss_pull_t if clip else None
    hi = boss_end_t if clip else None

    def in_window(t: float) -> bool:
        if lo is not None and t < lo - 0.05:
            return False
        if hi is not None and t > hi + 0.05:
            return False
        return True

    # ---- pass 2: collect events ----
    # event = (t, actor_id, glyph, text)
    events: list[tuple[float, str, str, str]] = []
    casts_by_actor: dict[str, Counter] = defaultdict(Counter)
    last_move: dict[str, tuple[float, float]] = {}

    def add(t: float, actor_id: str, glyph: str, text: str) -> None:
        events.append((t, actor_id, glyph, text))

    def tgt(ref: Ref) -> str:
        if not ref.is_valid():
            return "—"
        a = actors.get(ref.id)
        nm = (a.name if a else None) or ref.name or short(ref.id)
        return nm

    for r in rows:
        if len(r) < 3:
            continue
        kind = r[1].strip()
        t = parse_seconds(r[0])
        if t is None or not in_window(t):
            continue

        # subject actor id (most ops put the subject in field 2)
        if kind == "MOVE":
            subj_id = r[2].upper()
        else:
            subj_id = Ref(r[2]).id

        if kind == "ACT+" and len(r) > 20:
            if subj_id not in focal_ids:
                continue
            a = actors[subj_id]
            hp = f"{r[14]}/{r[15]}" if len(r) > 15 else "?"
            tgtable = "" if r[19].strip().lower() == "true" else " UNTARGETABLE"
            add(t, subj_id, "+", f"spawn  oid={a.oid} hp={hp} @{pos_str(r[11].split('/'))}{tgtable}")
        elif kind == "ACT-":
            if subj_id not in focal_ids:
                continue
            add(t, subj_id, "-", "despawn")
        elif kind == "CST+" and len(r) > 6:
            if subj_id not in focal_ids:
                continue
            aid, aname = parse_action(r[3])
            target = Ref(r[4])
            loc = "/".join(r[5].split("/"))
            try:
                total = float(r[6].split("/")[-1])
            except ValueError:
                total = 0.0
            self_cast = target.id == subj_id
            arrow = "(self)" if self_cast else f"-> {tgt(target)}"
            add(t, subj_id, "▶", f"cast {aname} ({aid}) [{total:.1f}s] {arrow}  @loc {pos_str(r[5].split('/'))}")
        elif kind == "CST!" and len(r) > 7:
            if subj_id not in focal_ids:
                continue
            aid, aname = parse_action(r[3])
            casts_by_actor[subj_id][(aid, aname)] += 1
            target = Ref(r[4])
            self_cast = target.id == subj_id
            arrow = "(self)" if self_cast else f"-> {tgt(target)}"
            extra = ""
            if args.targets and len(r) > 11:
                hit = [tgt(Ref(f.split("!")[0])) for f in r[11:] if f and Ref(f.split("!")[0]).is_valid()]
                if hit:
                    extra = f"  hits[{len(hit)}]: " + ", ".join(hit[:8])
            add(t, subj_id, "✦", f"fire {aname} ({aid}) {arrow}  @pos {pos_str(r[7].split('/'))}{extra}")
        elif kind in ("ATG+", "ATG-"):
            if subj_id not in focal_ids:
                continue
            add(t, subj_id, "◉", "targetable ON" if kind == "ATG+" else "targetable OFF")
        elif kind == "DIE+":
            if subj_id not in focal_ids:
                continue
            add(t, subj_id, "✝", "dies")
        elif kind == "MOVE" and args.moves and len(r) > 4:
            if subj_id not in focal_ids:
                continue
            try:
                x, _, z = (float(v) for v in r[3].split("/"))
            except ValueError:
                continue
            px, pz = last_move.get(subj_id, (1e9, 1e9))
            if abs(x - px) + abs(z - pz) >= 2.0:
                last_move[subj_id] = (x, z)
                add(t, subj_id, "→", f"move @({x:.1f}, {z:.1f})")
        elif kind == "STA+" and args.statuses and len(r) > 7:
            src = Ref(r[7])  # status SOURCE; we list it under the enemy that applied it
            if src.id not in focal_ids:
                continue
            sid, sname = parse_action(r[4])
            add(t, src.id, "+", f"applies status {sname} ({sid}) -> {tgt(Ref(r[2]))}")
        elif kind == "AIE+" and args.effects and len(r) > 7:
            src = Ref(r[6])
            if src.id not in focal_ids:
                continue
            aid, aname = parse_action(r[7])
            add(t, src.id, "✦", f"effect {aname} ({aid}) -> {tgt(Ref(r[2]))}")

    # ---- output ----
    print(f"file      : {args.log}")
    print(f"boss      : {boss.label}  maxHP={boss.max_hp}")
    helpers = sorted((actors[a] for a in focal_ids if a != boss.id), key=lambda a: a.oid)
    if helpers:
        seen = {}
        for h in helpers:
            seen.setdefault(h.oid, h.name)
        print("helpers   : " + ", ".join(f"{nm}({oid})" for oid, nm in seen.items()))
    if clip:
        dur = (hi if hi is not None else (events[-1][0] if events else lo)) - lo
        tail = f"boss HP→0/death @ +{boss_end_t - lo:.1f}s" if boss_end_t else "(no death in log)"
        print(f"window    : pull .. {tail}   duration {dur:.1f}s")
    else:
        print("window    : (full log, unclipped)")
    print()

    # per-actor cast summary
    print("cast counts per actor (CST! fires):")
    for aid in sorted(casts_by_actor, key=lambda a: (-sum(casts_by_actor[a].values()), a)):
        a = actors[aid]
        print(f"  [{a.label}]")
        for (cid, cname), c in sorted(casts_by_actor[aid].items(), key=lambda kv: -kv[1]):
            print(f"      {cid:<7} ×{c:<3} {cname}")
    if not casts_by_actor:
        print("  (none)")

    if args.summary_only:
        return

    # timeline
    events.sort(key=lambda e: e[0])
    if not events:
        print("\n(no events in window)")
        return
    base = lo if lo is not None else events[0][0]
    print()
    print(f"{'t(s)':>7}  {'actor':<22} event")
    print(f"{'-'*7}  {'-'*22} {'-'*48}")
    for t, actor_id, glyph, text in events:
        a = actors.get(actor_id)
        label = a.label if a else short(actor_id)
        print(f"{t - base:7.2f}  {label:<22} {glyph} {text}")


if __name__ == "__main__":
    main()
