# 05 — Zombori on a real machine: MacBook Air M1

Prerequisite: Zombori works in the VM (`docs/02` to `04`). This puts it on a 2020 MacBook Air M1
as the main system: power on, and after the Apple logo come the splash, the boot sound, the gate
and the tavern, with no macOS in between.

The M1 is the same ARM64 as the VM, so the components need no porting. Linux runs on it thanks to
[Asahi Linux](https://asahilinux.org) (drivers for Apple's hardware), packaged for Debian by the
[Bananas team](https://wiki.debian.org/InstallingDebianOn/Apple/M1). macOS stays on the disk, only
much smaller: Apple delivers the chip's firmware through it, so it must never be deleted.

```
Apple logo (firmware) ─► startup disk "zombori" ─► m1n1 + U-Boot ─► GRUB ─► Debian testing
  ─► splash ─► boot sound ─► gate ─► greetd ─► tavern
```

## 1. Make room in macOS

The installer can only give Debian space that is free inside macOS, and keeps some of it for
macOS itself (on a 245 GB disk with 87 GB free, it offered 49 GB). Delete files in your user folder
(`ncdu ~` shows what is large; leave `~/Library/Keychains` and `~/Library/Preferences`), empty the
Trash, and drop Time Machine's local snapshots (`sudo tmutil deletelocalsnapshots /`).

Do not use *Erase All Content and Settings* on a second-hand Mac before checking that it is not
tied to someone else's Apple ID (Find My): after erasing, the Mac asks for that account and
refuses to start without it. Installing Debian next to macOS does not touch that lock; only your
local macOS administrator account is needed.

## 2. Install Debian (from macOS, about 30 minutes)

In Terminal (the short address `bananas.debian.net` on the Debian wiki does not resolve):

```sh
curl -L https://bananas-archive.debian.net/install | sh
```

It is the official Asahi installer, pointed at Debian images. The answers:

| Question | Answer |
|---|---|
| Action | `r`, resize macOS, then the new size `min` (all the space it can take goes to Debian) |
| Action | `f`, install into the free space |
| OS | `Debian GNU/Linux Testing minimal (console)`: Zombori is the desktop, so no GNOME or KDE |
| Name | `zombori` (the name in the startup disk menu) |

Then follow its last screen exactly: wait 25 s after it shuts down, press and **hold** the power
button until "Loading startup options" appears, choose `zombori`, and in the *macOS Recovery*
dialog pick your macOS user and password. A terminal (`step2.sh`) asks for the credentials twice
and to confirm Permissive Security with `y`: that applies to Debian only, macOS keeps full
security. The Mac reboots into Debian, and from now on starts it by default.

## 3. First boot: user, name, network

The minimal image has no setup wizard. At `login:` type `root` (no password), then:

```sh
passwd                          # root's password
adduser daniel                  # your user (any name)
usermod -aG sudo daniel
hostnamectl set-hostname zombori
echo 127.0.1.1 zombori >> /etc/hosts   # without it, sudo warns "unable to resolve host"
dmesg -n 1                      # keep kernel messages (Wi-Fi driver chatter) off the console
```

There is no NetworkManager yet, only `wpa_supplicant`, so join Wi-Fi by hand once (an iPhone
hotspot works; rename the phone first if its name has a curly apostrophe, which the console can't
type):

```sh
IF=$(ls /sys/class/net | grep -v '^lo$'); echo $IF     # the Wi-Fi card, e.g. wld0
wpa_passphrase "NetworkName" > /etc/wpa_supplicant/wpa_supplicant-$IF.conf   # then type the password
systemctl enable --now wpa_supplicant@$IF
printf '[Match]\nName=wl*\n[Network]\nDHCP=yes\n' > /etc/systemd/network/25-wifi.network
systemctl enable --now systemd-networkd
echo "nameserver 1.1.1.1" > /etc/resolv.conf
ping -c 3 debian.org
```

Then install everything, including NetworkManager, and hand the network over to it:

```sh
apt update
apt install -y sudo curl git network-manager openssh-server asahi-platform alsa-utils \
  cargo build-essential cmake clang libclang-dev libasound2-dev pkg-config \
  greetd cage foot firefox-esr
systemctl disable --now wpa_supplicant@$IF systemd-networkd
rm /etc/systemd/network/25-wifi.network /etc/wpa_supplicant/wpa_supplicant-$IF.conf
systemctl restart NetworkManager
nmtui                           # Activate a connection → your network
```

`asahi-platform` brings sound support (and `speakersafetyd`, see below) to a minimal install.

## 4. Zombori

As your user:

```sh
git clone https://github.com/zombori-co/zombori && cd zombori
sudo install -m 644 system/apple-m1/asound.conf /etc/asound.conf
system/dev-install.sh
```

The clips (question, music, the tavern's question) are not in the repository. Copy them from the
Mac into `~/zombori/assets/sounds/local/` (`scp`, see below) and run `system/dev-install.sh`
again. Then `sudo reboot`.

### Working on it from the Mac over SSH

With both on the same network, find the Air's address with `ip -4 addr` and on the Mac:

```sh
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_zombori     # a key just for the Air
ssh-copy-id -i ~/.ssh/id_ed25519_zombori daniel@<address>
ssh -i ~/.ssh/id_ed25519_zombori daniel@<address>
```

Reading the services' logs needs `sudo`, or membership of the `systemd-journal` group
(`sudo usermod -aG systemd-journal daniel`); playing and recording over SSH needs the `audio` group.

## How the Air differs from the VM

Where the VM has one device, the Air has two, and each difference showed up as a failing service:

| | VM | MacBook Air M1 |
|---|---|---|
| Screen | one DRM card | `asahi` (the GPU, no outputs) and `apple-drm` (drives the panel, `eDP-1`). `screen` skips a card that refuses modesetting and takes the next one |
| Speakers | one card | card `AppleJ313`, **device 1** (`Speaker Playback Mux` = Secondary); device 0 is the headphone jack |
| Microphone | the Loopback card fed by `vm/mic.sh` | its own card `AppleJ313HPAI`: three microphones, raw 32-bit float, no gain |
| Speaker volume | ALSA mixer | locked by `speakersafetyd`, which guards the small speakers; it unlocks them a few ms into boot, so `audio` retries opening a device for up to 5 s |

`system/apple-m1/asound.conf` makes ALSA's "default" device right for the Air: playback on the
speakers, recording from the microphones converted from float (`lfloat`), mixed into one channel
with +30 dB (`route`), and resampled to the gate's 16 kHz (`plug`). On Asahi, PipeWire normally
does that mixing and gain (with proper beamforming); Zombori's components run as system services
before any user session exists, so they talk to ALSA directly. The card names are this model's
(J313); another Apple Silicon Mac needs its own names from `aplay -l` and `arecord -l`.

## Checking

```sh
systemctl --failed                                         # nothing of Zombori's
journalctl -b -u zombori-splash -u zombori-bootsound -u zombori-gate -o cat
aplay -D default /usr/share/sounds/alsa/Front_Center.wav   # the speakers
arecord -D default -f S16_LE -r 16000 -d 5 /tmp/t.wav && aplay /tmp/t.wav   # the microphones
```

The splash's journal says `screen: using /dev/dri/card2 (apple)`. Quiet room noise through the
microphones is around -45 dBFS, normal speech around -30 dBFS.

## Undoing it

Boot macOS (hold the power button, choose Macintosh HD) and remove the Debian partitions as the
[Asahi partitioning cheatsheet](https://asahilinux.org/docs/sw/partitioning-cheatsheet) describes.
Never reinstall or erase macOS on a Mac with someone else's Find My lock.
