# Changelog

## [0.2.0](https://github.com/zombori-co/zombori/compare/v0.1.0...v0.2.0) (2026-11-13)


### Features

* **bootsound:** play a boot jingle through alsa ([3448ac5](https://github.com/zombori-co/zombori/commit/3448ac5ff99bf42f675ba235f0bed80f6b67b06c))
* **gate:** ask a spoken question before the login prompt ([1b1064e](https://github.com/zombori-co/zombori/commit/1b1064ed69e91d728e1d465f38b154944d3a0e64))
* **gate:** ask a spoken question before the login prompt ([694e06e](https://github.com/zombori-co/zombori/commit/694e06e6f3776578cf9455ae8cdb584bfd23fb91))
* **tavern:** open programs by voice after the gate ([3f2db48](https://github.com/zombori-co/zombori/commit/3f2db481be99e9e674a40f1ce70002c76e94b09e))
* **tavern:** open programs by voice after the gate ([79a79f4](https://github.com/zombori-co/zombori/commit/79a79f4a8fdf413fc175b7c9baaa283021019349))


### Bug Fixes

* **system:** run on a MacBook Air M1 with separate display and sound devices ([ae965cd](https://github.com/zombori-co/zombori/commit/ae965cd47097c95344851763bafa8d1aee541081))
* **system:** run on a MacBook Air M1 with separate display and sound devices ([8e86f5a](https://github.com/zombori-co/zombori/commit/8e86f5a13d47ac8c9fc4b90f294cd7d1dd3f2073))

## 0.1.0 (2026-11-11)

First tagged state: milestone M3.

### Features

* **splash:** boot logo drawn straight to the GPU through DRM/KMS, fading in the logo and name and sweeping a glint across the sunglasses, started early in boot as a systemd service
* **theme:** shared palette, embedded pixel logo, 5x7 font and boot timeline
* **vm:** QEMU scripts for the builder and target VMs (ARM64, `hvf`, UEFI), with disk snapshots
* **docs:** foundation plan, hand install of Debian 13, dev loop and splash guide
