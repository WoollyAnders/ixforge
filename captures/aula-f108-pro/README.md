# AULA F108 Pro — USB captures (local only)

Drop your Wireshark `.pcapng` files here. See the **capture playbook** at the top of
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) for the full procedure.

`*.pcap` / `*.pcapng` are **git-ignored on purpose** — they can be large and device-identifying,
and IX Forge is clean-room: we commit the *derived* protocol notes in
[`docs/protocols/aula-f108-pro.md`](../../docs/protocols/aula-f108-pro.md), not the raw captures.
This README just keeps the folder present so there's an obvious place to save to.

Suggested files (one setting changed per capture):
`01-init`, `02-esc-red`, `03-esc-green`, `04-key1-red`, `05-all-blue`, `06-brightness`,
`07-effect` — later `08-lcd-image`, `09-macro`.
