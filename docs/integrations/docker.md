# docker

Docker images, stopped containers, unused volumes and the build cache, read through the Docker Engine API.

**Status:** stable · **Category:** Containers · **Verified:** 2026-10-25 against the Engine API docs (current API 1.56, oldest supported 1.40); kipple sends API v1.41 paths.

## How kipple finds Docker
`DOCKER_HOST` (`unix://`, `tcp://`, `npipe://`), then the current context from `~/.docker/config.json` → `contexts/meta/*/meta.json`, then `~/.docker/run/docker.sock` (Docker Desktop on macOS), `/var/run/docker.sock`, `$XDG_RUNTIME_DIR/docker.sock` (rootless), and `\\.\pipe\docker_engine` on Windows. TLS (`DOCKER_TLS_VERIFY`) and `ssh://` hosts aren't supported yet.

## Items and safety
| Item | Safety | Action |
|---|---|---|
| Build cache records not in use (one item) | Safe | `POST /build/prune` |
| Untagged images no container uses | Safe | `DELETE /images/{id}` (never forced) |
| Tagged images no container uses | Review | `DELETE /images/{id}` |
| Stopped containers (writable layer) | Review — may hold data | `DELETE /containers/{id}` |
| Volumes no container mounts | Review — may hold data | `DELETE /volumes/{name}` |
| Volumes mounted by any container | Keep | none |

Images used by any container, running or stopped, aren't listed. Image sizes subtract layers shared with other images. Compose labels attribute containers and volumes to their project folder.

Right before each action kipple asks Docker again whether the container is still stopped, or the image or volume still unused, and skips the item if not.

## Gotchas
- Removing a volume deletes its data for good; there's no undo for Docker actions.
- If Docker isn't running, the scan says so once and reports nothing for Docker.
