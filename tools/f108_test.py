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
import time
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


def send(h, payload, label=""):
    """Send one 64-byte payload as a Feature report (report id 0 prefixed).

    Returns the number of bytes written, or -1 on failure. Windows is strict
    about the buffer length matching the report's declared size, so a failure
    here (rather than the device ignoring us) is the thing to catch.
    """
    try:
        n = h.send_feature_report(bytes([0x00] + payload))
    except Exception as e:  # noqa: BLE001
        print(f"  {label}: EXCEPTION {e!r}")
        return -1
    if n is None or n < 0:
        print(f"  {label}: write FAILED (returned {n}); last error: {h.error()}")
    time.sleep(0.003)  # gentle pacing between reports
    return n


# The app sends these constantly while it's running (keeps a "software" session).
HEARTBEAT = [report([(0, 0x04), (1, 0x02)]), report([(0, 0x04), (1, 0xf5), (8, 0x09)])]


def apply_once(h, buf):
    results = []
    for i, r in enumerate(PREAMBLE):
        results.append(send(h, r, f"preamble[{i}]"))
    for i, r in enumerate(frame(buf)):
        results.append(send(h, r, f"data[{i}]"))
    results.append(send(h, COMMIT, "commit"))
    results.append(send(h, TRAILER, "trailer"))
    return results


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    mode = args[0] if args else "esc-red"
    hold = "--hold" in sys.argv
    if mode == "esc-red":
        buf = {1: (0xff, 0x00, 0x00)}
    elif mode == "all-red":
        buf = {p: (0xff, 0x00, 0x00) for p, idx in enumerate(INDEX_MAP) if idx != 0}
    elif mode == "off":
        buf = {}
    else:
        sys.exit("mode must be: esc-red | all-red | off")

    h = open_device()
    results = apply_once(h, buf)
    ok = sum(1 for n in results if n and n > 0)
    print(f"\nSent '{mode}': {ok}/{len(results)} writes accepted (each should return 65).")

    if hold:
        # Test the "software session" theory: keep the heartbeat alive and
        # periodically re-apply the frame for ~12s. WATCH THE KEYBOARD.
        print("Holding session ~12s (heartbeat + re-apply). WATCH THE KEYBOARD. Ctrl+C to stop.")
        end = time.time() + 12
        tick = 0
        try:
            while time.time() < end:
                for hb in HEARTBEAT:
                    send(h, hb)
                tick += 1
                if tick % 8 == 0:
                    apply_once(h, buf)
                time.sleep(0.05)
        except KeyboardInterrupt:
            pass

    h.close()
    if ok != len(results):
        print("Some writes were rejected — a report-length issue; paste the numbers.")


if __name__ == "__main__":
    main()
