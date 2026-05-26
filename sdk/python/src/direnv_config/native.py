from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml

from direnv_config.path import get_path


class NativeBackend:
    def __init__(self, store: Path) -> None:
        self._store = store

    def get(self, config: str, path: str | None = None) -> Any:
        active_file = self._store / config / ".active"
        if not active_file.exists():
            return None
        contents = active_file.read_text()
        data = yaml.safe_load(contents)
        if path is None:
            return data
        return get_path(data, path)

    def list_configs(self) -> list[str]:
        meta_file = self._store / ".meta"
        if not meta_file.exists():
            return []
        contents = meta_file.read_text()
        meta = yaml.safe_load(contents)
        if meta is None or not isinstance(meta, dict):
            return []
        configs = meta.get("configs", [])
        if not isinstance(configs, list):
            return []
        return [str(c) for c in configs]
