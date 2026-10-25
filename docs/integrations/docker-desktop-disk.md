# docker-desktop-disk

Docker Desktop's virtual disk file. **Report only** and **opt-in** (`[integrations] enable = ["docker-desktop-disk"]`).

**Status:** experimental · **Category:** Containers

## Paths (verify per Docker Desktop version)
- macOS: `~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw`
- Windows: `%LOCALAPPDATA%\Docker\wsl\disk\docker_data.vhdx`, `%LOCALAPPDATA%\Docker\wsl\data\ext4.vhdx`
- Linux (Docker Desktop): `~/.docker/desktop/vms/0/data/Docker.raw`

## Behaviour
Shows the file's logical size and how much disk it actually uses. Nothing is offered for removal: pruning inside Docker frees space in the image, and whether the host gets it back depends on the Docker Desktop version. Guided compaction is planned for v1.

It is opt-in because on recent macOS, reading another app's container folder can trigger a privacy prompt (decision 0009).
