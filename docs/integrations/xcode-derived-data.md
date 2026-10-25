# xcode-derived-data

Xcode's build products and indexes. macOS only.

**Status:** stable · **Category:** Mobile

## Paths
`defaults read com.apple.dt.Xcode IDECustomDerivedDataLocation`, else `~/Library/Developer/Xcode/DerivedData/`. One item per child folder (`<Project>-<hash>`, plus `ModuleCache.noindex`).

## Items and safety
**Safe** (`REGENERABLE`), quarantine. Each folder is attributed to its project from `info.plist` → `WorkspacePath`; folders for deleted projects are flagged. While Xcode runs, items are **Review** (`TOOL_RUNNING`).

Binary `info.plist` files aren't parsed yet, so those folders show without a project.
