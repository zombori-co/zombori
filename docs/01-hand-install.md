# 01 — Install Debian 13 by hand

You will build a Debian system from nothing onto an empty disk, one piece at a time.
Each step says **what** to run and **why**: this is where you learn what an OS is made of.

Two VMs are involved:

- **builder**: a stock Debian you use as a workshop (like booting a USB stick to install a PC).
- **target**: the empty disk you install onto. It becomes *your* system.

```
vm/fetch-base.sh     # once: download + verify the builder image
vm/builder.sh        # boot the builder; the target disk is /dev/vdb inside it
```

On the builder's very first boot, `systemd-firstboot` shows a small wizard ("Press any key to
proceed"). It asks for a timezone (press Enter to skip) and a root password. **Set a root password.**
The image's root account is locked, so skipping it leaves you unable to log in. Anything simple is
fine, because this is only the workshop machine. Then log in as `root` with that password.
To quit QEMU at any point: **Ctrl-A, then X**.

Locked out anyway? `rm vm/images/builder.qcow2 vm/images/builder-vars.fd` and run `vm/builder.sh` again. That gives you a fresh builder.

## 1. Tools for the job (in the builder)

```sh
apt update
apt install -y debootstrap gdisk dosfstools arch-install-scripts
```

- `debootstrap` downloads and unpacks a minimal Debian into a directory.
- `gdisk` (`sgdisk`) partitions disks. `dosfstools` makes FAT filesystems.
- `arch-install-scripts` gives `arch-chroot`. It comes from Arch and works on any distro.

## 2. Partition the target disk

```sh
lsblk                                   # vdb should be the empty 20G disk
sgdisk --zap-all /dev/vdb
sgdisk -n1:0:+512M -t1:ef00 -c1:EFI \
       -n2:0:0     -t2:8305 -c2:root /dev/vdb
lsblk /dev/vdb
```

- Partition 1 is the **EFI System Partition** (type `ef00`). It's a FAT32 partition, and UEFI firmware reads bootloaders from it.
- Partition 2 is the **root filesystem** (type `8305` = "Linux ARM64 root").

## 3. Filesystems and mounting

```sh
mkfs.vfat -F32 -n EFI /dev/vdb1
mkfs.ext4 -L root /dev/vdb2
mount /dev/vdb2 /mnt
mkdir -p /mnt/boot/efi
mount /dev/vdb1 /mnt/boot/efi
```

## 4. Lay down a minimal Debian

```sh
debootstrap --arch=arm64 trixie /mnt http://deb.debian.org/debian
```

Take a look around `/mnt` afterwards. It is a complete but tiny Debian: no kernel and no bootloader yet.

Now write `/etc/fstab`, which tells the new system which filesystems to mount at boot:

```sh
blkid /dev/vdb1 /dev/vdb2
cat > /mnt/etc/fstab <<EOF
UUID=$(blkid -s UUID -o value /dev/vdb2)  /          ext4  defaults,errors=remount-ro  0 1
UUID=$(blkid -s UUID -o value /dev/vdb1)  /boot/efi  vfat  umask=0077                  0 2
EOF
cat /mnt/etc/fstab
```

Filesystems are referred to by **UUID**, never by device name. Here the disk is `/dev/vdb`, but
when the target boots on its own it becomes `/dev/vda`. An fstab line with `/dev/vdb1` would make
boot wait 90 seconds for a disk that never appears, then drop into emergency mode.
(`genfstab -U` from `arch-install-scripts` is meant to do this automatically, but in testing it
wrote `/dev/vdb1` for the EFI partition, which is exactly that failure.)

## 5. Step inside the new system

```sh
arch-chroot /mnt
```

From now on, commands run *inside* the new system. `arch-chroot` also mounts `/proc`, `/sys` and `/dev`
so that package scripts work.

## 6. Identity

```sh
echo zombori > /etc/hostname
printf '127.0.0.1\tlocalhost\n127.0.1.1\tzombori\n' > /etc/hosts
ln -sf /usr/share/zoneinfo/Europe/Zurich /etc/localtime
```

## 7. Kernel and bootloader

```sh
apt update
apt install -y linux-image-arm64 grub-efi-arm64
```

The kernel package also brings `initramfs-tools`. The **initramfs** is a small early root filesystem
that loads the drivers needed to find the real root. Later, your boot splash will run from it.

```sh
sed -i 's/^GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT=""/' /etc/default/grub
sed -i 's/^GRUB_CMDLINE_LINUX=.*/GRUB_CMDLINE_LINUX="console=ttyAMA0 console=tty0"/' /etc/default/grub
grub-install --target=arm64-efi --efi-directory=/boot/efi --removable
update-grub
```

- `--removable` installs GRUB at the fallback path `EFI/BOOT/BOOTAA64.EFI`. Firmware always looks there,
  so no UEFI boot entry is needed.
- The `console=` options send kernel messages to both the serial port (your terminal) and the screen.
  The last one listed, `tty0`, becomes the main console.
- Leaving out `quiet` shows every boot message for now. Your splash will hide them later.

## 8. Networking, SSH, basics

```sh
apt install -y systemd-resolved openssh-server sudo locales alsa-utils less vim
printf '[Match]\nName=en*\n\n[Network]\nDHCP=yes\n' > /etc/systemd/network/20-wired.network
systemctl enable systemd-networkd
dpkg-reconfigure locales            # pick en_US.UTF-8 (or your own)
```

`systemd-networkd` is a small network manager configured with plain files. This one file says
"use DHCP on any Ethernet interface".

## 9. Users

```sh
passwd                              # root password
useradd -m -G sudo,audio,video -s /bin/bash <you>
passwd <you>
```

The `audio` and `video` groups give your user access to the sound card and the GPU, which the boot sound and splash need.

## 10. Leave, unmount, power off

```sh
exit                                # leave the chroot
umount -R /mnt
poweroff
```

## 11. Boot your system

```sh
vm/target.sh
```

A window opens and boot messages appear there and in the terminal. Log in as your user, then from the Mac:

```sh
ssh -p 2223 <you>@localhost
```

If it works, save a snapshot right away (with the VM shut down):

```sh
vm/snapshot.sh save fresh-install
```

## When something goes wrong

- **Drops to the UEFI shell:** GRUB isn't at `EFI/BOOT/BOOTAA64.EFI` on the ESP. Boot the builder, mount
  both partitions again (step 3), `arch-chroot`, and re-run step 7.
- **"Enter root password for system maintenance" after a ~90s pause:** a line in `/etc/fstab` points
  at a disk that doesn't exist. Usually it's a `/dev/vdbX` name instead of a UUID. Fix it from the
  builder (mount as in step 3, then edit `/mnt/etc/fstab`).
- **Kernel starts but can't find root:** check `/etc/fstab`, and `blkid` against it.
- **Starting over:** `rm vm/images/target.qcow2 vm/images/target-vars.fd` and begin again at step 1.
