#!/usr/bin/env python3
"""Stream a color via the AULA F108 Pro EFFECT path (0420) -- the path that
actually drives the LEDs live (see docs/protocols/aula-f108-pro.md).

Streams the captured effect frame, recolored per LED index from a target buffer,
continuously so the board holds a stable color. This is the reference for the
Rust driver's live-display path.

Run on native Windows, keyboard WIRED, AULA fully closed:
    pip install hid
    python tools/f108_effect.py --color all          # whole board red
    python tools/f108_effect.py --color esc           # ONLY Esc red, rest off
    python tools/f108_effect.py --color esc --secs 8
Watch: does only the intended key(s) light? Stable while streaming?
"""
import sys
import time
import hid

VID, PID, IFACE = 0x0C45, 0x800A, 3
READ_LEN = 65
TICK = 0.033

CONNECT = [
    '04180000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', 'R', '04280000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', 'R',
    '00015a1a07010826090003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000aa55', 'R', '04020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', 'R',
]

EFFECT = [
    '04200000000000000800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', 'R', '01fe000002fe000003fe000004fe7e4f05fe000006ec902007fe000008fe000009febfca0afe00000bfe00000cfe00000dfe440070fe000071fe000073fe0000', '13fef9f914fe000015fe000016fe000017fe600018fe000019fe00001afe00001bfe00001c7f29291dfe00001ef97f711ffe00006786322374fe000075fe0000',
    '76e2ce5620fe000021fe000022fe00007a72491125fed60026fe000027fe000028e2160c29fe00002afe00002bfe00002caf161e2dfe00002efe00002ffe0000', '30a2937f31fe000043fe000077fe00fe78fe000079efe58b32fe000033fe000034fe00007bfe000037fe000038fe000039fe00003ae2a7683bfe00003cfe0000', '3dfe00003efe7f003ffe000040fe000041fe000042fe000055fe000044fe981145fe000046fe000049fe00004afe00004bfe00004cfe00004db121214efe0000', '4ffe000050af2f5f51fe000052fe000053fe000054fe000065fefe0056bb8e8e57fe000058fe00006afe00005bfe00005cfe00005d9b651e5efe00005f862556',
    '60fe000061fe000062fe000063fe000064fe7c3f66fe000068ea8d5469fe00000000000000000000000000000000000000000000000000000000000000000000', '00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', '04020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', 'R',
    '04020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000',
]

ALL_INDICES = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 112, 113, 115, 116, 117, 118, 119, 120, 121, 122, 123]


def recolor(hexstr, buf):
    """Set each 4-byte [idx,R,G,B] record's color from buf[idx] (default off)."""
    b = bytearray.fromhex(hexstr)
    for i in range(0, 64, 4):
        idx = b[i]
        if idx != 0:
            r, g, bl = buf.get(idx, (0, 0, 0))
            b[i+1], b[i+2], b[i+3] = r, g, bl
    return b.hex()

def frame_ops(buf):
    """The effect frame, recolored to the target buffer."""
    out = []
    for op in EFFECT:
        if op == "R" or op[:2] in ("04", "80") or op == "00"*64:
            out.append(op)
        else:
            out.append(recolor(op, buf))
    return out

def open_device():
    devs = hid.enumerate(VID, PID)
    if not devs:
        sys.exit("No device found. Wired? App closed?")
    chosen = next((d for d in devs if d["interface_number"] == IFACE), None) or devs[-1]
    print(f"Using iface={chosen['interface_number']}")
    h = hid.device(); h.open_path(chosen["path"]); return h

def send(h, hexstr):
    try:
        n = h.send_feature_report(bytes([0x00]) + bytes.fromhex(hexstr))
    except Exception as e:
        print(f"  WRITE EXCEPTION {e!r}"); return -1
    if n is None or n < 0:
        print(f"  write FAILED ({n}); {h.error()}")
    time.sleep(TICK); return n

def read(h):
    for attempt in range(3):
        try:
            data = h.get_feature_report(0, READ_LEN)
        except Exception:
            if attempt == 2: return b""
            time.sleep(0.004); continue
        time.sleep(TICK); return bytes(data or b"")
    return b""

def replay(h, ops):
    for op in ops:
        read(h) if op == "R" else send(h, op)

def main():
    argv = sys.argv[1:]
    def opt(name, d): return argv[argv.index(name)+1] if name in argv else d
    color = opt("--color", "all")
    secs = float(opt("--secs", "15"))
    if color == "all":
        buf = {i: (0xff, 0, 0) for i in ALL_INDICES}
    elif color == "esc":
        buf = {0x01: (0xff, 0, 0)}
    elif color == "off":
        buf = {}
    else:
        sys.exit("--color must be all | esc | off")

    h = open_device()
    ops = frame_ops(buf)
    print(f"Connect + stream '{color}' via effect path for ~{secs:.0f}s. WATCH THE BOARD.")
    replay(h, CONNECT)
    end = time.time() + secs; n = 0
    try:
        while time.time() < end:
            replay(h, ops); n += 1
            print(f"  frame {n}")
    except KeyboardInterrupt:
        pass
    h.close()
    print(f"\nDone ({n} frames, color={color}). Only the intended keys lit? Stable?")

if __name__ == "__main__":
    main()
