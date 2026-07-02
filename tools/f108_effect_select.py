#!/usr/bin/env python3
"""Select an onboard effect on the AULA F108 Pro (decoded from capture 07).

Effect select is a single command (no streaming): connect handshake, then
0418 / 0413[8]=01 / [effect_id, ff, ..., 01, speed, brightness, ...aa55] / 04f0.
The board then animates the effect on its own MCU, so it keeps running after this
tool exits (that's how we confirm it's onboard, not host-streamed).

Run on native Windows, keyboard WIRED, AULA fully closed:
    pip install hid
    python tools/f108_effect_select.py 1                 # effect id 1
    python tools/f108_effect_select.py 3 --speed 5 --brightness 5
Sweep ids 1..18 and note which animation each is, to build the effect map.
"""
import sys, time, hid

VID, PID, IFACE = 0x0C45, 0x800A, 3
READ_LEN = 65
TICK = 0.033


def report(pairs):
    r = [0] * 64
    for i, v in pairs:
        r[i] = v
    return r


CONFIG = report([(1, 0x01), (2, 0x5a), (3, 0x1a), (4, 0x07), (5, 0x01),
                 (6, 0x08), (7, 0x26), (8, 0x09), (10, 0x03), (62, 0xaa), (63, 0x55)])


def effect_packet(effect_id, speed, brightness):
    # byte0=effect id, byte1=0xff(enable), byte8=0x01, byte9=speed, byte10=brightness, aa55 commit
    return report([(0, effect_id), (1, 0xff), (8, 0x01), (9, speed), (10, brightness),
                   (62, 0xaa), (63, 0x55)])


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


def cmd(h, payload, ack=True):
    send(h, payload)
    if ack:
        read(h)


def connect(h):
    cmd(h, report([(0, 0x04), (1, 0x18)]))
    cmd(h, report([(0, 0x04), (1, 0x28), (8, 0x01)]))
    cmd(h, CONFIG)
    cmd(h, report([(0, 0x04), (1, 0x02)]))


def select_effect(h, effect_id, speed, brightness):
    cmd(h, report([(0, 0x04), (1, 0x18)]))
    cmd(h, report([(0, 0x04), (1, 0x13), (8, 0x01)]))
    cmd(h, effect_packet(effect_id, speed, brightness))
    send(h, report([(0, 0x04), (1, 0xf0)]))  # trailer, no ack


def main():
    argv = sys.argv[1:]
    def opt(name, d):
        return int(argv[argv.index(name) + 1]) if name in argv else d
    speed = opt("--speed", 5)
    brightness = opt("--brightness", 3)

    if "--sweep" in argv:
        rng = [int(a) for a in argv if not a.startswith("--")]
        lo, hi = (rng + [1, 18])[:2] if len(rng) >= 2 else (1, 18)
        h = open_device()
        connect(h)
        out = "effects-map.txt"
        f = open(out, "a")
        print(f"Sweeping effect ids {lo}..{hi}. For each: watch the board (press keys for\n"
              f"reactive ones), then type its name + Enter. Blank = skip, 'quit' = stop.\n"
              f"Saved to {out}.\n")
        for eid in range(lo, hi + 1):
            select_effect(h, eid, speed, brightness)
            resp = input(f"effect {eid} (0x{eid:02x}) — what animation is it? ").strip()
            if resp.lower() == "quit":
                break
            f.write(f"{eid}\t0x{eid:02x}\t{resp or '(none)'}\n"); f.flush()
        h.close()
        print(f"\nDone. Send me {out}.")
        return

    ids = [int(a) for a in argv if not a.startswith("--")]
    effect_id = ids[0] if ids else 1
    h = open_device()
    print(f"Selecting effect id={effect_id} (0x{effect_id:02x}), speed={speed}, brightness={brightness}...")
    connect(h)
    select_effect(h, effect_id, speed, brightness)
    h.close()
    print(f"\nSent effect {effect_id}. Which animation is it? Does it keep running now?")


if __name__ == "__main__":
    main()
