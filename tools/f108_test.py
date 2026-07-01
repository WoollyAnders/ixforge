#!/usr/bin/env python3
"""On-hardware check for the AULA F108 Pro (Sonix) RGB protocol.

Replays the exact byte sequence the official AULA app uses, decoded from the
owner's USB capture (`captures/aula-f108-pro/04-connect-esc-red.pcapng`), so you
can confirm the protocol lights the real keyboard without building the Rust app.

Key finding from capture 04: this is a **command/ACK handshake**. Every control
command (the `04 xx` / `80` reports) is a Feature SET_REPORT that the app FOLLOWS
with a Feature GET_REPORT (read) to drain the device's ACK. The 8 data reports
stream without reads; there's one read after the commit. If you only ever write
(as an earlier version did), the device accepts every SET_REPORT at the USB layer
("16/16 accepted") but its command parser desyncs and the frame never latches —
so nothing lights. Reading the ACK after each control command is the fix.

There is also a one-time connect handshake (`04 28`, a config packet ending in
`aa 55`) the app sends before any frame; we replay it in init_session().

Setup (native Windows, or Linux with HID access):
    pip install hid          # cython-hidapi
Run with the keyboard WIRED and the official AULA app CLOSED:
    python tools/f108_test.py esc-red     # Esc -> red
    python tools/f108_test.py all-red     # whole board -> red
    python tools/f108_test.py off         # all off
    python tools/f108_test.py esc-red --hold   # hold session ~12s, watch board
    python tools/f108_test.py esc-red --quiet  # suppress per-report logging
"""
import sys
import time
import hid

VID, PID, IFACE = 0x0C45, 0x800A, 3
# Windows HidD_GetFeature needs the buffer sized to the device's FULL feature
# report length. Our writes are 65 bytes (report-id byte + 64 data) and succeed,
# so the report length is 65 — read the same size or HidD_GetFeature errors.
READ_LEN = 65
# The official app paces EVERY report ~36 ms apart (writes, reads, and each of
# the 8 data reports). This device completes each control transfer slowly; if we
# stream reports faster it can't keep up and the frame never renders (board stays
# dark though every write "succeeds"). Match the cadence.
TICK = 0.033

VERBOSE = True

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


# --- One-time connect handshake (capture frames 1095-1151) ---------------
# Config packet the app sends right after 04 28; committed with aa 55.
CONFIG = report([
    (1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
    (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x03), (62, 0xaa), (63, 0x55),
])

# --- Per-apply control preamble / commit / trailer (capture ~frame 3195) --
PRE_ATTN = report([(0, 0x04), (1, 0x18)])       # "attention"
PRE_MODE = report([(0, 0x04), (1, 0x13), (8, 0x01)])
PRE_80 = report([(0, 0x80), (9, 0x05), (14, 0xaa), (15, 0x55)])
PRE_F0 = report([(0, 0x04), (1, 0xf0)])         # no ACK read follows this one
PRE_LATCH = report([(0, 0x04), (1, 0x23), (8, 0x09)])
COMMIT = report([(62, 0xaa), (63, 0x55)])
TRAILER = report([(0, 0x04), (1, 0xf0)])
HEARTBEAT = report([(0, 0x04), (1, 0x02)])


def frame(buffer):
    """buffer: {slot: (r, g, b)} -> list of 8 x 64-byte reports."""
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
    """Write one 64-byte payload as a Feature report (report id 0 prefixed)."""
    try:
        n = h.send_feature_report(bytes([0x00] + payload))
    except Exception as e:  # noqa: BLE001
        print(f"  {label}: WRITE EXCEPTION {e!r}")
        return -1
    if n is None or n < 0:
        print(f"  {label}: write FAILED (returned {n}); last error: {h.error()}")
    elif VERBOSE:
        print(f"  {label}: wrote {n}")
    time.sleep(TICK)
    return n


def read(h, label=""):
    """Read the device's ACK (Feature GET_REPORT, report id 0). Returns bytes.

    The device is a request/response lock-step: after each control command it
    parks a response that must be drained before it will accept the next write.
    """
    for attempt in range(3):
        try:
            data = h.get_feature_report(0, READ_LEN)
        except Exception as e:  # noqa: BLE001
            if attempt == 2:
                print(f"  {label}: READ EXCEPTION {e!r}")
                return b""
            time.sleep(0.004)
            continue
        b = bytes(data or b"")
        if VERBOSE:
            print(f"  {label}: read {len(b)}B  {b[:8].hex()}")
        time.sleep(TICK)
        return b
    return b""


def cmd(h, payload, ack=True, label=""):
    """Send a control report and (optionally) drain the device's ACK read."""
    send(h, payload, label)
    if ack:
        read(h, label + " ack")


def init_session(h):
    """One-time connect handshake (capture frames 1095-1151)."""
    print("Init handshake:")
    cmd(h, PRE_ATTN, label="init 0418")
    cmd(h, report([(0, 0x04), (1, 0x28), (8, 0x01)]), label="init 0428")
    cmd(h, CONFIG, label="init config")
    cmd(h, HEARTBEAT, label="init 0402")


def apply_once(h, buf):
    """Switch to static per-key and upload one frame (capture ~frame 3195)."""
    print("Apply frame:")
    cmd(h, HEARTBEAT, label="hb")
    cmd(h, PRE_ATTN, label="pre 0418")
    cmd(h, PRE_MODE, label="pre 0413")
    cmd(h, PRE_80, label="pre 80")
    cmd(h, PRE_F0, ack=False, label="pre 04f0")
    cmd(h, PRE_ATTN, label="pre 0418b")
    cmd(h, PRE_LATCH, label="pre 0423")
    for i, r in enumerate(frame(buf)):
        send(h, r, f"data[{i}]")  # data reports stream without ACK reads
    cmd(h, COMMIT, label="commit")
    send(h, TRAILER, "trailer")  # 04 f0, no ACK read


def main():
    global VERBOSE
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    mode = args[0] if args else "esc-red"
    hold = "--hold" in sys.argv
    VERBOSE = "--quiet" not in sys.argv

    if mode == "esc-red":
        buf = {1: (0xff, 0x00, 0x00)}
    elif mode == "all-red":
        buf = {p: (0xff, 0x00, 0x00) for p, idx in enumerate(INDEX_MAP) if idx != 0}
    elif mode == "off":
        buf = {}
    else:
        sys.exit("mode must be: esc-red | all-red | off")

    h = open_device()
    init_session(h)
    apply_once(h, buf)
    print(f"\nSent '{mode}'. WATCH THE KEYBOARD.")

    if hold:
        print("Holding session ~12s (heartbeat + re-apply). Ctrl+C to stop.")
        end_at = time.time() + 12
        tick = 0
        try:
            while time.time() < end_at:
                cmd(h, HEARTBEAT, label="hb")
                tick += 1
                if tick % 8 == 0:
                    apply_once(h, buf)
                time.sleep(0.05)
        except KeyboardInterrupt:
            pass

    h.close()


if __name__ == "__main__":
    main()
