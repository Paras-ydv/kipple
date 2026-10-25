# android-avd

Android Virtual Devices (emulators).

**Status:** stable · **Category:** Mobile

## Paths
AVD home: `ANDROID_AVD_HOME` → `ANDROID_USER_HOME/avd` → `~/.android/avd`. Each emulator is `<name>.ini` plus `<name>.avd/` (or the `path=` in the `.ini`). Display name, device and system image come from `<name>.avd/config.ini` (`avd.ini.displayname`, `hw.device.name`, `image.sysdir.1`).

## Items and safety
- Each emulator: **Review** (holds apps, data and saved state). Action: `avdmanager delete avd -n <name>` from `<sdk>/cmdline-tools/latest/bin` (or PATH); otherwise quarantine of both the `.ini` and the folder.
- Its Quick Boot snapshots: separate **Review** item; the emulator cold boots once and makes new ones.
- An emulator that is running (`qemu-system-*`/`emulator` with `-avd <name>` or `@<name>`) is **Keep**, and its snapshots aren't offered.
