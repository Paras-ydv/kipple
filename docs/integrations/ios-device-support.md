# ios-device-support

Debug symbols Xcode copies from each device OS version (iOS, watchOS, tvOS, visionOS). macOS only.

**Status:** stable · **Category:** Mobile

## Paths
`~/Library/Developer/Xcode/{iOS,watchOS,tvOS,visionOS} DeviceSupport/*`, one item per OS version folder (`17.4 (21E219)`, `iPhone16,2 18.1 (22B83)`).

## Items and safety
Older versions are **Safe** (`REDOWNLOADABLE`: Xcode copies them again when a device with that version connects). The newest version per platform is **Review**. Action: quarantine.
