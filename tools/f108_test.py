#!/usr/bin/env python3
"""Quick on-hardware check for the AULA F108 Pro (Sonix) RGB protocol.

This replays the exact byte sequence IX Forge's `sonix` driver produces (decoded
from the owner's USB capture) so you can confirm the protocol lights the real
keyboard without building the Rust app.

Setup (native Windows, or Linux with HID access):
    pip install hid          # cython-hidapi
Run with the keyboard WIRED and the official AULA app CLOSED:
    python tools/f108_test.py esc-red     # Esc -> red
    python tools/f108_test.py all-red     # whole board -> red
    python tools/f108_test.py off         # all off
"""
import sys
import hid

VID, PID, IFACE = 0x0C45, 0x800A, 3

# Which led_index sits in each of the 128 frame slots (0 = no LED). From capture.
INDEX_MAP = [
    0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x00,0x00,
    0x00,0x00,0x00,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1a,0x1b,0x1c,0x1d,0x1e,0x1f,
    0x20,0x21,0x22,0x00,0x00,0x25,0x26,0x27,0x28,0x29,0x2a,0x2b,0x2c,0x2d,0x2e,0x2f,
    0x30,0x31,0x32,0x33,0x34,0x00,0x00,0x37,0x38,0x39,0x3a,0x3b,0x3c,0x3d,0x3e,0x3f,
    0x40,0x41,0x42,0x43,0x44,0x45,0x46,0x00,0x00,0x49,0x4a,0x4b,0x4c,0x4d,0x4e,0x4f,
    0x50,0x51,0x52,0x53,0x54,0x55,0x56,0x57,0x58,0x00,0x00,0x5b,0x5c,0x5d,0x5e,0x5f,
    0x60,0x61,0x62,0x63,0x64,0x65,0x66,0x67,0x68,0x69,0x6a,0x00,0x00,0x00,0x00,0x00,
    0x70,0x71,0x00,0x73,0x74,0x75,0x76,0x77,0x78,0x79,0x7a,0x7b,0x00,0x00,0x00,0x00,
]


def report(pairs):
    r = [0] * 64
    for i, v in pairs:
        r[i] = v
    return r


PREAMBLE = [
    report([(0, 0x04), (1, 0x18)]),
    report([(0, 0x04), (1, 0x13), (8, 0x01)]),
    report([(0, 0x80), (9, 0x05), (14, 0xaa), (15, 0x55)]),
    report([(0, 0x04), (1, 0xf0)]),
    report([(0, 0x04), (1, 0x18)]),
    report([(0, 0x04), (1, 0x23), (8, 0x09)]),
]
COMMIT = report([(62, 0xaa), (63, 0x55)])
TRAILER = report([(0, 0x04), (1, 0xf0)])


def frame(buffer):
    """buffer: {led_index: (r, g, b)} -> list of 8 x 64-byte reports."""
    reports = []
    for rep_i in range(8):
        rep = [0] * 64
        for s in range(16):
            pos = rep_i * 16 + s
            idx = INDEX_MAP[pos]
            off = s * 4
            rep[off] = idx
            if idx != 0 and pos in buffer:
                rep[off + 1], rep[off + 2], rep[off + 3] = buffer[pos]
        reports.append(rep)
    return reports


def open_device():
    devs = hid.enumerate(VID, PID)
    if not devs:
        sys.exit(f"No device {VID:#06x}:{PID:#06x} found. Wired? App closed?")
    print("Interfaces seen:")
    for d in devs:
        print(f"  iface={d['interface_number']} usage_page={d['usage_page']:#06x} path={d['path']}")
    chosen = next((d for d in devs if d["interface_number"] == IFACE), None) or devs[-1]
    print(f"Using: iface={chosen['interface_number']} path={chosen['path']}")
    h = hid.device()
    h.open_path(chosen["path"])
    return h


def send(h, payload):
    # First byte = report ID (0 for this device).
    h.send_feature_report(bytes([0x00] + payload))


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "esc-red"
    if mode == "esc-red":
        buf = {1: (0xff, 0x00, 0x00)}
    elif mode == "all-red":
        buf = {p: (0xff, 0x00, 0x00) for p, idx in enumerate(INDEX_MAP) if idx != 0}
    elif mode == "off":
        buf = {}
    else:
        sys.exit("mode must be: esc-red | all-red | off")

    h = open_device()
    for r in PREAMBLE:
        send(h, r)
    for r in frame(buf):
        send(h, r)
    send(h, COMMIT)
    send(h, TRAILER)
    h.close()
    print(f"Sent '{mode}'. Look at the keyboard.")


if __name__ == "__main__":
    main()
