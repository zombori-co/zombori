# 04 — The tavern: opening programs by voice

Prerequisite: `docs/03-gate.md` is done (the gate runs at boot and hears you).

After the gate, you are logged in automatically and the man stays with you, now in a tavern. He
asks which room you want; say a room's name (**«браузер»**) and he bows, the picture fades, and the
room's program opens full screen. Close the program and you are back in the tavern.

```
gate («музика») ─► greetd logs you in ─► tavern: "which room?" ─► «браузер» ─► Firefox, full screen
                                              ▲                                        │
                                              └──────────── close Firefox ◄────────────┘
```

This is stage 1 of 3: the tavern is still a wooden wall behind him. Stage 2 adds the walk into a
3D tavern with musicians and a door; stage 3 the corridor and a door for each room.

## How it fits together

| Part | Where |
|---|---|
| Automatic login after the gate, as you, on the first console | greetd: `system/session/greetd.toml`, `greetd-zombori.conf` |
| Your session: starts the tavern | `system/session/zombori-session` |
| The tavern: asks, listens, opens rooms | `crates/tavern` (reuses the gate's 3D man, voice and recognizer) |
| The rooms: words → name → program | `/etc/zombori/rooms.toml` (from `system/session/rooms.toml`) |
| Programs full screen | cage, a kiosk Wayland compositor |

The tavern draws straight to the GPU like the gate. When a room opens it hands the screen to cage,
waits for the program to close, and takes the screen back.

## 1. Install the packages

Inside the VM:

```sh
sudo apt install -y greetd cage firefox-esr
```

## 2. The question clip

The man's question in the tavern ("which room?") is your own clip, like the gate's:

```
assets/sounds/local/room-question.wav     16-bit PCM WAV (afconvert as in docs/03-gate.md)
```

Without it he just listens.

## 3. Install and reboot

```sh
/mnt/zombori/system/dev-install.sh
sudo reboot
```

With greetd and cage installed, `dev-install.sh` now also writes `/etc/greetd/config.toml` (with
your user name), turns off the plain login prompt on the first console, and enables greetd.

After the gate: the tavern. Say «браузер». greetd starts your session, so what the tavern heard
(and any error) is in greetd's log:

```sh
journalctl -b -u greetd --no-pager | grep tavern
```

## Rooms

`/etc/zombori/rooms.toml` lists the rooms (it is installed only once, so edit it freely):

```toml
[[room]]
name = "Браузер"
words = ["браузер", "інтернет", "browser"]
command = ["firefox-esr"]
```

Any of `words`, spoken, opens `command`. Swap `command` for another program (your own browser,
later), or add another `[[room]]`. The tavern reads the file when it starts.

## Login, and getting back in

- At boot, saying «музика» logs you in **without a password** (you chose this). Anyone who hears
  the answer can do the same at the keyboard. SSH still asks for your password.
- If the tavern ever exits, greetd shows a text login that asks for your password.
- The other consoles still work: **Ctrl+Alt+F2** for a plain login, and SSH.
- To go back to the plain login prompt after the gate:
  `sudo systemctl disable greetd && sudo systemctl enable getty@tty1`.

## Testing on the Mac

```sh
cargo test -p tavern -p gate                         # rooms, word matching, scene
cargo run -p gate --example preview -- /tmp/t --tavern   # the tavern's stages as PNG frames
```
