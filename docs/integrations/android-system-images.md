# android-system-images

Android OS images that emulators boot from.

**Status:** stable · **Category:** Mobile

## Paths
SDK root: `ANDROID_HOME` → `ANDROID_SDK_ROOT` → `~/Library/Android/sdk` (macOS), `~/Android/Sdk` (Linux), `%LOCALAPPDATA%\Android\Sdk` (Windows). Images live in `<sdk>/system-images/<api>/<tag>/<abi>/`.

## Items and safety
- Image no emulator's `image.sysdir.1` points to: **Safe** (`UNUSED_BY_ANY_EMULATOR`). Action: `sdkmanager --uninstall "system-images;<api>;<tag>;<abi>"`, else quarantine.
- Image used by an emulator: **Keep** (`USED_BY_EMULATOR`, with the emulator names).
