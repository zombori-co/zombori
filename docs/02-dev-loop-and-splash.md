# 02 — Dev loop and boot splash

Prerequisite: `docs/01-hand-install.md` is done and `vm/target.sh` boots your system.

At the end of this guide your own Rust code runs on every boot: the Zombori logo fades in.

## 1. Tools inside the VM

Boot with `vm/target.sh`, log in (in the window or with `ssh -p 2223 <you>@localhost`), then:

```sh
sudo apt install -y build-essential curl ca-certificates
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. ~/.cargo/env
rustc --version
```

`build-essential` provides the C linker that Rust uses to produce the final executable.

## 2. Mount the repo inside the VM

`vm/target.sh` exports the repo from the Mac with mount tag `zombori` (a 9p share). Mount it, and
add it to `/etc/fstab` so it is there after every boot:

```sh
sudo mkdir -p /mnt/zombori
echo 'zombori /mnt/zombori 9p trans=virtio,version=9p2000.L,msize=262144,nofail 0 0' | sudo tee -a /etc/fstab
sudo systemctl daemon-reload
sudo mount /mnt/zombori
ls /mnt/zombori
```

`nofail` means a missing share doesn't stop boot. That matters later, when the system boots
somewhere without the Mac.

The loop from now on: **edit on the Mac, build and run in the VM.** No copying.

## 3. Build and install the splash

```sh
/mnt/zombori/system/dev-install.sh
sudo systemctl start zombori-splash     # watch the VM window
```

What `dev-install.sh` does: builds the workspace (output in `~/.cache/zombori-target`, because builds
on the share are slow), installs the binary as `/usr/local/bin/zombori-splash`, installs
`system/splash/zombori-splash.service` and enables it.

Reboot (`sudo reboot`) and the splash plays during boot.

## 4. Get the boot screens out of the way

Before the splash you will see the firmware's countdown bar and GRUB's blue menu. Each waits
5 seconds. Then kernel messages flash by. Turn all three off:

```sh
# GRUB: boot immediately and show no menu (hold Shift or Esc during boot to get it back)
sudo sed -i 's/^GRUB_TIMEOUT=.*/GRUB_TIMEOUT=0\nGRUB_TIMEOUT_STYLE=hidden/' /etc/default/grub
# Kernel: only show serious messages
sudo sed -i 's/^GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT="quiet loglevel=3"/' /etc/default/grub
sudo update-grub

# UEFI firmware: no countdown before starting the boot entry
sudo apt install -y efibootmgr
sudo efibootmgr -t 0
```

`efibootmgr` edits the firmware's own settings (UEFI variables) from Linux. On a real PC this is
the same thing you would change in the BIOS setup screen.

In testing, this took the splash from ~13 s after power-on to under 3 s.

## Where to go from here

- Change the look: `assets/logo/` (logo and mockup) and `crates/theme` (colors, timing, font).
  `cargo test` on the Mac checks that the splash still matches `assets/logo/bootscreen.png`.
- After a change: run `dev-install.sh` again, then `sudo systemctl start zombori-splash`.
- Logs: `journalctl -b -u zombori-splash`.
