#!/usr/bin/env python3
"""Find the CLEANEST LED-matrix warmup for the AULA F108 Pro.

f108_min.py proved 1 captured effect frame powers the matrix, but that flashes
the captured rainbow. This tries simpler warmups so the driver stays clean:

    --mode bare    : connect + bare `04 20` command      + static apply
    --mode target  : connect + `04 20` + static-format frame of the TARGET color
                     (would warm up AND display in one step -- no rainbow flash)
    --mode none    : connect + static apply only (control: expected dark)

Run on native Windows, keyboard WIRED, AULA app CLOSED:
    pip install hid
    python tools/f108_warmup.py --mode bare
    python tools/f108_warmup.py --mode target
Tell me which modes light the board (and whether any rainbow flashes).
"""
import sys
import time
import hid

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


def report(pairs):
    r = [0] * 64
    for i, v in pairs:
        r[i] = v
    return r


CONFIG = report([(1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
                 (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x03), (62, 0xaa), (63, 0x55)])
EFFECT_PRE = report([(0, 0x04), (1, 0x20), (8, 0x08)])   # effect/live-mode preamble
PRE = [
    report([(0, 0x04), (1, 0x18)]),
    report([(0, 0x04), (1, 0x13), (8, 0x01)]),
    report([(0, 0x80), (9, 0x05), (14, 0xaa), (15, 0x55)]),
    report([(0, 0x04), (1, 0xf0)]),
    report([(0, 0x04), (1, 0x18)]),
    report([(0, 0x04), (1, 0x23), (8, 0x09)]),
]
PRE_ACK = [True, True, True, False, True, True]
COMMIT = report([(62, 0xaa), (63, 0x55)])
TRAILER = report([(0, 0x04), (1, 0xf0)])
ZEROS = report([])
HEARTBEAT = report([(0, 0x04), (1, 0x02)])


def build_frame(buf):
    reps = []
    for ri in range(8):
        rep = [0] * 64
        for s in range(16):
            pos = ri * 16 + s
            idx = INDEX_MAP[pos]
            off = s * 4
            rep[off] = idx
            if idx != 0 and pos in buf:
                rep[off + 1], rep[off + 2], rep[off + 3] = buf[pos]
        reps.append(rep)
    return reps


def open_device():
    devs = hid.enumerate(VID, PID)
    if not devs:
        sys.exit("No device found. Wired? App closed?")
    chosen = next((d for d in devs if d["interface_number"] == IFACE), None) or devs[-1]
    print(f"Using iface={chosen['interface_number']}")
    h = hid.device()
    h.open_path(chosen["path"])
    return h


def send(h, payload):
    try:
        n = h.send_feature_report(bytes([0x00] + list(payload)))
    except Exception as e:  # noqa: BLE001
        print(f"  WRITE EXCEPTION {e!r}")
        return -1
    if n is None or n < 0:
        print(f"  write FAILED ({n}); {h.error()}")
    time.sleep(TICK)
    return n


def read(h):
    for attempt in range(3):
        try:
            data = h.get_feature_report(0, READ_LEN)
        except Exception:  # noqa: BLE001
            if attempt == 2:
                return b""
            time.sleep(0.004)
            continue
        time.sleep(TICK)
        return bytes(data or b"")
    return b""


def connect(h):
    send(h, PRE[0]); read(h)                                  # 04 18
    send(h, report([(0, 0x04), (1, 0x28), (8, 0x01)])); read(h)
    send(h, CONFIG); read(h)
    send(h, HEARTBEAT); read(h)


def static_apply(h, buf):
    send(h, HEARTBEAT); read(h)
    for rep, ack in zip(PRE, PRE_ACK):
        send(h, rep)
        if ack:
            read(h)
    for rep in build_frame(buf):
        send(h, rep)
    send(h, COMMIT); read(h)
    send(h, TRAILER)


def main():
    argv = sys.argv[1:]
    mode = argv[argv.index("--mode") + 1] if "--mode" in argv else "bare"
    buf = {p: (0xff, 0, 0) for p, idx in enumerate(INDEX_MAP) if idx != 0}  # all red

    h = open_device()
    print(f"Connect + warmup mode='{mode}' + static all-red.")
    connect(h)

    if mode == "bare":
        # just the effect/live-mode preamble, no data
        send(h, EFFECT_PRE); read(h)
    elif mode == "target":
        # effect preamble + a static-format frame of the target color + terminator
        send(h, EFFECT_PRE); read(h)
        for rep in build_frame(buf):
            send(h, rep)
        send(h, ZEROS)
    elif mode == "none":
        pass
    else:
        sys.exit("mode must be: bare | target | none")

    static_apply(h, buf)
    h.close()
    print(f"\nDone (mode={mode}). Did the board light? Any rainbow flash, or straight to red?")


if __name__ == "__main__":
    main()
