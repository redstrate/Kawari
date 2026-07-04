"""
extract_obfuscation.py

Run INSIDE IDA (Alt+F7 -> select this file) with the target ffxiv_dx11 client
loaded. Auto-derives Kawari's packet-obfuscation constants and dumps the six
scrambler tables straight into the Kawari source tree.

Outputs (under KAWARI_ROOT):
  resources/data/constants.yml
      OBFUSCATION_ENABLED_MODE
      OBFUSCATION_TABLE_RADIXES  [3]
      OBFUSCATION_TABLE_MAX      [3]
  resources/data/scrambler/
      table0.bin  table1.bin  table2.bin
      midtable.bin  daytable.bin  opcodekeytable.bin

This is a trimmed Python/IDA port of perchbird's Unscrambler
(CodeGenerator + DataGenerator, https://github.com/perchbirdd/Unscrambler),
keeping only what Kawari's `core/src/packet/scrambler.rs` consumes.

Difference from the reference, on purpose: each table's `max` is derived from
the table's byte span in .rdata (distance to the next table) instead of from an
`imul` immediate. The client compiles `value % max` differently per constant --
e.g. `% 72` becomes `lea [r+r*8]; shl ,3` (x72) rather than `imul ,72` -- so the
imul-only heuristic silently reads the wrong max (119 instead of 72) and would
dump a wrong-sized table. The span-based method matches Kawari upstream exactly.
"""

import os

import idaapi
import idautils
import idc
import ida_bytes
import ida_funcs

# --------------------------------------------------------------------------
# Config
# --------------------------------------------------------------------------
KAWARI_ROOT = r"F:\FFXIVPluginSRCs\Kawari"

# Signature inside PacketDispatcher_OnReceivePacket, at the opcode-key-table
# lookup. Single '?' = one wildcard byte.
PACKET_DISPATCHER_SIG = "? ? ? 2B C8 ? 8B ? 8A ? ? ? ? 41 81"

# A memory-operand displacement in this range is a .rdata table RVA
# (7 hex digits), as opposed to a small stack/struct offset.
_RVA_LO, _RVA_HI = 0x1000000, 0xFFFFFFF

# Element -> byte multipliers for each dumped table.
_MUL_TABLE = 4   # table0/1/2 are int32
_MUL_MID = 8     # midtable entries are 8 bytes
_MUL_DAY = 4     # daytable is int32
_MUL_OKT = 4     # opcodekeytable is int32

IMAGE_BASE = idaapi.get_imagebase()


# --------------------------------------------------------------------------
# Low-level IDA helpers
# --------------------------------------------------------------------------
def _text_bounds():
    for seg in idautils.Segments():
        if idc.get_segm_name(seg) == ".text":
            return idc.get_segm_start(seg), idc.get_segm_end(seg)
    raise RuntimeError(".text segment not found")


def _find_sig(sig):
    lo, hi = _text_bounds()
    ea = ida_bytes.find_bytes(sig, range_start=lo, range_end=hi)
    if ea == idc.BADADDR:
        raise RuntimeError("PacketDispatcher signature not found: %s" % sig)
    return ea


def _func_insns(start):
    fn = ida_funcs.get_func(start)
    if not fn:
        return []
    out, ea = [], fn.start_ea
    while ea < fn.end_ea and ea != idc.BADADDR:
        out.append(ea)
        ea = idc.next_head(ea)
    return out


def _disp_rva(ea):
    """RVA of a .rdata memory-operand displacement on this instruction, or None."""
    for i in range(3):
        if idc.get_operand_type(ea, i) == idaapi.o_displ:
            v = idc.get_operand_value(ea, i)
            if _RVA_LO <= v <= _RVA_HI:
                return v
    return None


def _last_imm(ea):
    """Value of the last immediate operand (imul's imm is operand 2), or None."""
    for i in range(2, -1, -1):
        if idc.get_operand_type(ea, i) == idaapi.o_imm:
            return idc.get_operand_value(ea, i)
    return None


def _near_target(ea):
    """Branch/call target of a near operand, or None."""
    for i in range(2):
        if idc.get_operand_type(ea, i) == idaapi.o_near:
            return idc.get_operand_value(ea, i)
    return None


def _preceding_imul(insns, k):
    """Nearest `imul reg, imm` immediate before index k."""
    while k > 0:
        k -= 1
        if idc.print_insn_mnem(insns[k]) == "imul" and _last_imm(insns[k]) is not None:
            return _last_imm(insns[k])
    return None


# --------------------------------------------------------------------------
# Derivation
# --------------------------------------------------------------------------
def derive_constants():
    R = {
        "TableOffsets": [0, 0, 0], "TableRadixes": [0, 0, 0],
        "TableMax": [0, 0, 0], "TableSizes": [0, 0, 0],
        "OpcodeKeyTableOffset": None, "OpcodeKeyTableSize": None,
        "MidTableOffset": None, "MidTableSize": None,
        "DayTableOffset": None, "DayTableSize": None,
        "ObfuscationEnabledMode": None, "UnknownObfuscationInitOpcode": None,
        "Derive": None, "DeriveSubs": [],
    }

    dispatcher = ida_funcs.get_func(_find_sig(PACKET_DISPATCHER_SIG))
    if not dispatcher:
        raise RuntimeError("no function around the signature")
    ins = _func_insns(dispatcher.start_ea)

    # The first RVA-displacement `mov` in the dispatcher is the opcode-key-table
    # lookup. Walk backwards for its size (imul imm), the Derive() call, and the
    # two cmp immediates (obfuscation mode + unknown-init opcode).
    for idx, ea in enumerate(ins):
        if idc.print_insn_mnem(ea) != "mov":
            continue
        rva = _disp_rva(ea)
        if rva is None:
            continue
        R["OpcodeKeyTableOffset"] = rva
        j = idx
        while j > 0:
            j -= 1
            if idc.print_insn_mnem(ins[j]) == "imul" and _last_imm(ins[j]) is not None:
                R["OpcodeKeyTableSize"] = _last_imm(ins[j])
                break
        while j > 0:
            j -= 1
            if idc.print_insn_mnem(ins[j]) == "call":
                der = _near_target(ins[j])
                if der is None:
                    continue
                R["Derive"] = der
                for il in _func_insns(der):
                    if idc.print_insn_mnem(il) in ("jz", "jnz", "je", "jne"):
                        t = _near_target(il)
                        if t is not None:
                            R["DeriveSubs"].append(t)
                break
        while j > 0:
            j -= 1
            if idc.print_insn_mnem(ins[j]) == "cmp" and _last_imm(ins[j]) is not None:
                if R["ObfuscationEnabledMode"] is None:
                    R["ObfuscationEnabledMode"] = _last_imm(ins[j])
                    continue
                R["UnknownObfuscationInitOpcode"] = _last_imm(ins[j])
                break
        break

    if R["Derive"] is None or len(R["DeriveSubs"]) < 3:
        raise RuntimeError("failed to locate Derive() and its 3 set-blocks")
    subs = R["DeriveSubs"][:3]  # dispatch order == set 0, 1, 2

    derive = ida_funcs.get_func(R["Derive"])
    dins = _func_insns(R["Derive"])

    # Mid table (imul [rva]) and day table (add [rva]); each preceded by an
    # imul imm giving the element count.
    for k, ea in enumerate(dins):
        mnem = idc.print_insn_mnem(ea)
        rva = _disp_rva(ea)
        if rva is None:
            continue
        if mnem == "imul" and R["MidTableOffset"] is None:
            R["MidTableOffset"] = rva
            R["MidTableSize"] = _preceding_imul(dins, k)
        elif mnem == "add" and R["DayTableOffset"] is None:
            R["DayTableOffset"] = rva
            R["DayTableSize"] = _preceding_imul(dins, k)

    # Each set-block accesses exactly one table; its radix is the imul immediate
    # just before that table access.
    non_table = {R["MidTableOffset"], R["DayTableOffset"], R["OpcodeKeyTableOffset"]}
    sub_sorted = sorted(subs)
    for set_i, blk_start in enumerate(subs):
        larger = [s for s in sub_sorted if s > blk_start]
        blk_end = min(larger) if larger else derive.end_ea
        for k, ea in enumerate(dins):
            if not (blk_start <= ea < blk_end):
                continue
            if idc.print_insn_mnem(ea) != "mov":
                continue
            rva = _disp_rva(ea)
            if rva is None or rva in non_table:
                continue
            R["TableOffsets"][set_i] = rva
            kk = k
            while kk > 0:
                kk -= 1
                if dins[kk] < blk_start:
                    break
                if idc.print_insn_mnem(dins[kk]) == "imul" and _last_imm(dins[kk]) is not None:
                    R["TableRadixes"][set_i] = _last_imm(dins[kk])
                    break
            break

    # `max` from each table's .rdata span (distance to the next known table),
    # divided by the radix. Robust against the lea/shl-encoded modulo.
    all_off = sorted([
        R["TableOffsets"][0], R["TableOffsets"][1], R["TableOffsets"][2],
        R["MidTableOffset"], R["DayTableOffset"], R["OpcodeKeyTableOffset"],
    ])
    for x in range(3):
        to = R["TableOffsets"][x]
        radix = R["TableRadixes"][x]
        if radix in (None, 0):
            raise RuntimeError("radix for table %d not found" % x)
        nxt = min(o for o in all_off if o > to)
        elems = (nxt - to) // 4
        R["TableMax"][x] = elems // radix
        R["TableSizes"][x] = radix * R["TableMax"][x]

    return R


# --------------------------------------------------------------------------
# Output
# --------------------------------------------------------------------------
def dump_tables(R, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    specs = [
        ("table0.bin", R["TableOffsets"][0], R["TableSizes"][0] * _MUL_TABLE),
        ("table1.bin", R["TableOffsets"][1], R["TableSizes"][1] * _MUL_TABLE),
        ("table2.bin", R["TableOffsets"][2], R["TableSizes"][2] * _MUL_TABLE),
        ("midtable.bin", R["MidTableOffset"], R["MidTableSize"] * _MUL_MID),
        ("daytable.bin", R["DayTableOffset"], R["DayTableSize"] * _MUL_DAY),
        ("opcodekeytable.bin", R["OpcodeKeyTableOffset"], R["OpcodeKeyTableSize"] * _MUL_OKT),
    ]
    for name, rva, nbytes in specs:
        data = ida_bytes.get_bytes(IMAGE_BASE + rva, nbytes)
        if data is None or len(data) != nbytes:
            raise RuntimeError("failed reading %s (%d bytes @ rva 0x%X)" % (name, nbytes, rva))
        with open(os.path.join(out_dir, name), "wb") as f:
            f.write(data)
        print("  wrote %-20s %7d bytes  (rva 0x%X)" % (name, nbytes, rva))


def update_constants_yaml(path, mode, radixes, table_max):
    with open(path, "r", encoding="utf-8") as f:
        lines = f.read().split("\n")

    scalars = {"OBFUSCATION_ENABLED_MODE": mode}
    lists = {"OBFUSCATION_TABLE_RADIXES": list(radixes),
             "OBFUSCATION_TABLE_MAX": list(table_max)}

    out, i, seen = [], 0, set()
    while i < len(lines):
        line = lines[i]
        key = line.split(":", 1)[0].strip()
        if key in scalars:
            out.append("%s: %s" % (key, scalars[key]))
            seen.add(key)
            i += 1
            continue
        if key in lists and line.strip() == key + ":":
            out.append(key + ":")
            seen.add(key)
            i += 1
            while i < len(lines) and lines[i].lstrip().startswith("- "):
                i += 1
            out.extend("- %s" % v for v in lists[key])
            continue
        out.append(line)
        i += 1

    missing = (set(scalars) | set(lists)) - seen
    if missing:
        raise RuntimeError("constants.yml keys not found: %s" % ", ".join(sorted(missing)))

    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(out))


def main():
    print("=== Kawari obfuscation extractor ===")
    R = derive_constants()
    print("ObfuscationEnabledMode = %d" % R["ObfuscationEnabledMode"])
    print("TableRadixes           = %s" % R["TableRadixes"])
    print("TableMax               = %s" % R["TableMax"])
    print("TableOffsets           = [%s]" % ", ".join(hex(o) for o in R["TableOffsets"]))
    print("Mid 0x%X x%d | Day 0x%X x%d | OKT 0x%X x%d" % (
        R["MidTableOffset"], R["MidTableSize"],
        R["DayTableOffset"], R["DayTableSize"],
        R["OpcodeKeyTableOffset"], R["OpcodeKeyTableSize"]))

    scrambler_dir = os.path.join(KAWARI_ROOT, "resources", "data", "scrambler")
    constants_yml = os.path.join(KAWARI_ROOT, "resources", "data", "constants.yml")

    print("Dumping tables -> %s" % scrambler_dir)
    dump_tables(R, scrambler_dir)

    print("Updating %s" % constants_yml)
    update_constants_yaml(constants_yml, R["ObfuscationEnabledMode"],
                          R["TableRadixes"], R["TableMax"])
    print("Done.")


main()
