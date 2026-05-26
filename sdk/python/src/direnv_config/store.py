from __future__ import annotations

import hashlib
import os
from pathlib import Path


class StoreNotFoundError(Exception):
    pass


def state_dir() -> Path:
    xdg = os.environ.get("XDG_STATE_HOME")
    if xdg:
        return Path(xdg) / "direnv-config"
    home = Path.home()
    return home / ".local" / "state" / "direnv-config"


def path_to_hash(directory: str) -> str:
    stripped = directory.lstrip("/")
    name = stripped.replace("/", "-")

    if len(name) <= 200:
        return name

    digest = hashlib.sha256(directory.encode()).hexdigest()
    return f"{name[:200]}-{digest[:8]}"


def store_path(directory: str) -> Path:
    return state_dir() / path_to_hash(directory)


def find_current_store(start_dir: str | None = None) -> Path:
    current = Path(start_dir) if start_dir else Path.cwd()
    current = current.resolve()

    while True:
        sp = store_path(str(current))
        if sp.exists():
            return sp
        parent = current.parent
        if parent == current:
            break
        current = parent

    raise StoreNotFoundError(
        f"no store found for {start_dir or Path.cwd()} "
        f"(searched all parent directories). Run `dc init` first."
    )
