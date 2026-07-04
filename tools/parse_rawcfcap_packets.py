#!/usr/bin/env python3
"""Dump IPC packets from a Chronofoil raw capture.

This parser targets the raw stream described by chronofoil_raw.ksy:

  capture_frame*:
    u8 protocol, u8 direction, then an FFXIV frame

  FFXIV frame:
    16 bytes prefix
    u64 frame timestamp
    u32 frame size
    u16 frame protocol
    u16 packet count
    u8 version
    u8 compression
    u16 unknown
    u32 decompressed length
    packet[packet count]

  packet:
    u32 size, u32 src, u32 dst, u16 type, u16 padding
    if type == 3:
      u16 unknown, u16 opcode, u16 padding, u16 server id, u32 timestamp, u32 padding
      payload bytes

By default only IPC packets are printed as TSV:
  timestamp<TAB>opcode<TAB>length<TAB>raw_hex
"""

from __future__ import annotations

import argparse
import struct
import sys
from dataclasses import dataclass
from pathlib import Path


DEFAULT_CAPTURE = "5cdb7c33-90a3-4ce6-a580-45ac4d07c1a2.rawcfcap"

FRAME_HEADER_SIZE = 40
PACKET_HEADER_SIZE = 16
IPC_HEADER_SIZE = 16


@dataclass(frozen=True)
class IpcPacket:
    frame_index: int
    packet_index: int
    direction: int
    protocol: int
    frame_timestamp: int
    ipc_timestamp: int
    opcode: int
    src: int
    dst: int
    packet_size: int
    payload: bytes
    packet_bytes: bytes


class ParseError(ValueError):
    pass


def u8(data: bytes, offset: int) -> int:
    return data[offset]


def u16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def i16(data: bytes, offset: int) -> int:
    return struct.unpack_from("<h", data, offset)[0]


def u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def u64(data: bytes, offset: int) -> int:
    return struct.unpack_from("<Q", data, offset)[0]


def looks_like_frame_stream(data: bytes, offset: int) -> bool:
    if offset < 0 or offset + 2 + FRAME_HEADER_SIZE > len(data):
        return False

    frame_start = offset + 2
    frame_size = u32(data, frame_start + 24)
    packet_count = u16(data, frame_start + 30)
    compression = u8(data, frame_start + 33)

    if frame_size < FRAME_HEADER_SIZE:
        return False
    if frame_start + frame_size > len(data):
        return False
    if packet_count > 4096:
        return False
    if compression not in (0, 1, 2):
        return False

    return True


def detect_stream_offset(data: bytes) -> int:
    """Detect whether this is a pure .rawcfcap stream or a full legacy file.

    The checked-in C# RawCaptureReader also supports a legacy file with a
    persistent header and capture header before the frame stream. This sample
    starts at offset 0, matching chronofoil_raw.ksy, but the fallback makes the
    tool useful for both forms.
    """
    if looks_like_frame_stream(data, 0):
        return 0

    if len(data) >= 32:
        persistent_size = u32(data, 0)
        candidates = [
            persistent_size + 28,
            254 + 28,
            256 + 28,
        ]
        for candidate in candidates:
            if looks_like_frame_stream(data, candidate):
                return candidate

    raise ParseError("could not find a valid raw capture frame stream")


def iter_ipc_packets(data: bytes, start_offset: int) -> tuple[list[IpcPacket], int]:
    packets: list[IpcPacket] = []
    pos = start_offset
    frame_index = 0

    while pos < len(data):
        if pos + 2 > len(data):
            break
        if i16(data, pos) == -1:
            break
        if pos + 2 + FRAME_HEADER_SIZE > len(data):
            raise ParseError(f"truncated capture frame at offset {pos}")

        capture_protocol = u8(data, pos)
        direction = u8(data, pos + 1)
        frame_start = pos + 2

        frame_timestamp = u64(data, frame_start + 16)
        frame_size = u32(data, frame_start + 24)
        frame_protocol = u16(data, frame_start + 28)
        packet_count = u16(data, frame_start + 30)
        compression = u8(data, frame_start + 33)

        if compression != 0:
            raise ParseError(
                f"unsupported compressed frame at offset {frame_start}: compression={compression}"
            )
        if frame_size < FRAME_HEADER_SIZE:
            raise ParseError(f"invalid frame size {frame_size} at offset {frame_start}")
        frame_end = frame_start + frame_size
        if frame_end > len(data):
            raise ParseError(f"frame at offset {frame_start} extends past end of file")

        packet_pos = frame_start + FRAME_HEADER_SIZE
        for packet_index in range(packet_count):
            if packet_pos + PACKET_HEADER_SIZE > frame_end:
                raise ParseError(
                    f"truncated packet header in frame {frame_index} packet {packet_index}"
                )

            packet_size = u32(data, packet_pos)
            src = u32(data, packet_pos + 4)
            dst = u32(data, packet_pos + 8)
            packet_type = u16(data, packet_pos + 12)

            if packet_size < PACKET_HEADER_SIZE:
                raise ParseError(
                    f"invalid packet size {packet_size} in frame {frame_index} packet {packet_index}"
                )
            packet_end = packet_pos + packet_size
            if packet_end > frame_end:
                raise ParseError(
                    f"packet in frame {frame_index} packet {packet_index} extends past frame"
                )

            packet_bytes = data[packet_pos:packet_end]
            if packet_type == 3:
                ipc_header_start = packet_pos + PACKET_HEADER_SIZE
                payload_start = ipc_header_start + IPC_HEADER_SIZE
                if payload_start > packet_end:
                    raise ParseError(
                        f"truncated IPC header in frame {frame_index} packet {packet_index}"
                    )

                opcode = u16(data, ipc_header_start + 2)
                ipc_timestamp = u32(data, ipc_header_start + 8)
                payload = data[payload_start:packet_end]
                packets.append(
                    IpcPacket(
                        frame_index=frame_index,
                        packet_index=packet_index,
                        direction=direction,
                        protocol=capture_protocol or frame_protocol,
                        frame_timestamp=frame_timestamp,
                        ipc_timestamp=ipc_timestamp,
                        opcode=opcode,
                        src=src,
                        dst=dst,
                        packet_size=packet_size,
                        payload=payload,
                        packet_bytes=packet_bytes,
                    )
                )

            packet_pos = packet_end

        pos = frame_end
        frame_index += 1

    return packets, frame_index


def format_hex(data: bytes, max_bytes: int | None) -> str:
    if max_bytes is not None and len(data) > max_bytes:
        return data[:max_bytes].hex() + "..."
    return data.hex()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Parse a Chronofoil .rawcfcap file and dump IPC packets."
    )
    parser.add_argument(
        "capture",
        nargs="?",
        default=DEFAULT_CAPTURE,
        help=f"raw capture file, default: {DEFAULT_CAPTURE}",
    )
    parser.add_argument(
        "--raw",
        choices=("payload", "packet"),
        default="payload",
        help="hex source: IPC payload only, or full packet including packet/IPC headers",
    )
    parser.add_argument(
        "--timestamp",
        choices=("ipc", "frame"),
        default="ipc",
        help="timestamp column source: IPC header seconds, or frame timestamp",
    )
    parser.add_argument(
        "--opcode",
        type=lambda value: int(value, 0),
        action="append",
        help="only dump this opcode; accepts decimal or 0x-prefixed hex; repeatable",
    )
    parser.add_argument(
        "--max-bytes",
        type=int,
        default=None,
        help="truncate raw_hex to this many bytes",
    )
    parser.add_argument(
        "--header",
        action="store_true",
        help="print a TSV header row",
    )
    parser.add_argument(
        "--meta",
        action="store_true",
        help="also print parse summary to stderr",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    path = Path(args.capture)
    data = path.read_bytes()

    try:
        start_offset = detect_stream_offset(data)
        packets, frame_count = iter_ipc_packets(data, start_offset)
    except ParseError as exc:
        print(f"parse error: {exc}", file=sys.stderr)
        return 2

    opcode_filter = set(args.opcode or [])
    if args.meta:
        print(
            f"file={path} bytes={len(data)} stream_offset={start_offset} "
            f"frames={frame_count} ipc_packets={len(packets)}",
            file=sys.stderr,
        )

    if args.header:
        print("timestamp\topcode\tlength\traw_hex")

    for packet in packets:
        if opcode_filter and packet.opcode not in opcode_filter:
            continue
        raw = packet.payload if args.raw == "payload" else packet.packet_bytes
        timestamp = packet.ipc_timestamp if args.timestamp == "ipc" else packet.frame_timestamp
        print(
            f"{timestamp}\t0x{packet.opcode:04x}\t{len(raw)}\t{format_hex(raw, args.max_bytes)}"
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
