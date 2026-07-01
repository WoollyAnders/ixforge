#!/usr/bin/env python3
"""Test PERSISTENCE via save-inside-a-live-session on the AULA F108 Pro.

Hypothesis: the static (0413/0423) write only "takes" while the board is in
software-display mode, which the 0420 effect stream establishes. So this:
  1. connects,
  2. streams the effect frame for a couple seconds (locks software mode; board
     shows the color live),
  3. sends the static apply *while the session is live* (the onboard save),
  4. streams a little more, then exits.
Then unplug ~3s and replug (no AULA): if it boots in the color, the save stuck.

Run on native Windows, keyboard WIRED, AULA fully closed:
    pip install hid
    python tools/f108_persist.py 00ff00     # green
THEN unplug ~3s, replug (no AULA): boots green?
"""
import sys, time, hid

VID, PID, IFACE = 0x0C45, 0x800A, 3
READ_LEN = 65
TICK = 0.033

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
EFFECT_LAYOUT = [
    [0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x70,0x71,0x73],
    [0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1a,0x1b,0x1c,0x1d,0x1e,0x1f,0x67,0x74,0x75],
    [0x76,0x20,0x21,0x22,0x7a,0x25,0x26,0x27,0x28,0x29,0x2a,0x2b,0x2c,0x2d,0x2e,0x2f],
    [0x30,0x31,0x43,0x77,0x78,0x79,0x32,0x33,0x34,0x7b,0x37,0x38,0x39,0x3a,0x3b,0x3c],
    [0x3d,0x3e,0x3f,0x40,0x41,0x42,0x55,0x44,0x45,0x46,0x49,0x4a,0x4b,0x4c,0x4d,0x4e],
    [0x4f,0x50,0x51,0x52,0x53,0x54,0x65,0x56,0x57,0x58,0x6a,0x5b,0x5c,0x5d,0x5e,0x5f],
    [0x60,0x61,0x62,0x63,0x64,0x66,0x68,0x69,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],
]


def report(pairs):
    r = [0] * 64
    for i, v in pairs:
        r[i] = v
    return r


CONFIG = report([(1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
                 (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x03), (62, 0xaa), (63, 0x55)])
EFFECT_PRE = report([(0, 0x04), (1, 0x20), (8, 0x08)])
HEARTBEAT = report([(0, 0x04), (1, 0x02)])
STATIC_PRE = [
    (report([(0, 0x04), (1, 0x18)]), True),
    (report([(0, 0x04), (1, 0x13), (8, 0x01)]), True),
    (report([(0, 0x80), (9, 0x05), (14, 0xaa), (15, 0x55)]), True),
    (report([(0, 0x04), (1, 0xf0)]), False),
    (report([(0, 0x04), (1, 0x18)]), True),
    (report([(0, 0x04), (1, 0x23), (8, 0x09)]), True),
]
COMMIT = report([(62, 0xaa), (63, 0x55)])
TRAILER = report([(0, 0x04), (1, 0xf0)])


def effect_frame(rgb):
    reps = []
    for row in EFFECT_LAYOUT:
        rep = [0] * 64
        for s, idx in enumerate(row):
            if idx != 0:
                off = s * 4
                rep[off] = idx
                rep[off + 1], rep[off + 2], rep[off + 3] = rgb
        reps.append(rep)
    return reps


def static_frame(rgb):
    reps = []
    for ri in range(8):
        rep = [0] * 64
        for s in range(16):
            pos = ri * 16 + s
            idx = INDEX_MAP[pos]
            off = s * 4
            rep[off] = idx
            if idx != 0:
                rep[off + 1], rep[off + 2], rep[off + 3] = rgb
        reps.append(rep)
    return reps


def open_device():
    devs = hid.enumerate(VID, PID)
    if not devs:
        sys.exit("No device found. Wired? App closed?")
    chosen = next((d for d in devs if d["interface_number"] == IFACE), None) or devs[-1]
    print(f"Using iface={chosen['interface_number']}")
    h = hid.device(); h.open_path(chosen["path"]); return h


def send(h, payload):
    try:
        n = h.send_feature_report(bytes([0x00] + payload))
    except Exception as e:  # noqa: BLE001
        print(f"  WRITE EXCEPTION {e!r}"); return -1
    if n is None or n < 0:
        print(f"  write FAILED ({n}); {h.error()}")
    time.sleep(TICK); return n


def read(h):
    for attempt in range(3):
        try:
            data = h.get_feature_report(0, READ_LEN)
        except Exception:  # noqa: BLE001
            if attempt == 2:
                return b""
            time.sleep(0.004); continue
        time.sleep(TICK); return bytes(data or b"")
    return b""


def cmd(h, payload, ack):
    send(h, payload)
    if ack:
        read(h)


def stream_effect(h, frame):
    send(h, EFFECT_PRE); read(h)
    for rep in frame:
        send(h, rep)
    send(h, report([]))       # terminator
    send(h, HEARTBEAT); read(h)
    send(h, HEARTBEAT)


def static_save(h, frame):
    for payload, ack in STATIC_PRE:
        cmd(h, payload, ack)
    for rep in frame:
        send(h, rep)
    cmd(h, COMMIT, True)
    send(h, TRAILER)


def main():
    hexc = next((a for a in sys.argv[1:] if not a.startswith("--")), "00ff00")
    rgb = (int(hexc[0:2], 16), int(hexc[2:4], 16), int(hexc[4:6], 16))
    h = open_device()
    ef = effect_frame(rgb)
    sf = static_frame(rgb)

    print(f"1) Locking software mode: streaming #{hexc} effect frames...")
    for _ in range(8):
        stream_effect(h, ef)
    print(f"2) Saving #{hexc} via static apply (inside the live session)...")
    static_save(h, sf)
    print("3) Streaming a bit more to let it settle...")
    for _ in range(4):
        stream_effect(h, ef)
    h.close()
    print(f"\nDone. UNPLUG ~3s, REPLUG (no AULA). Boots #{hexc}? Or old color / rainbow?")


if __name__ == "__main__":
    main()
