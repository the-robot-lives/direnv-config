from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any

import yaml

from direnv_config.native import NativeBackend


class CliBackend:
    def __init__(self, store: Path, dc_binary: str = "dc") -> None:
        self._store = store
        self._dc_binary = dc_binary
        self._native = NativeBackend(store)

    def get(self, config: str, path: str | None = None) -> Any:
        cmd = [self._dc_binary, "get", config]
        if path is not None:
            cmd.append(path)
        cmd.append("--raw")
        result = subprocess.run(cmd, capture_output=True, text=True, check=False)
        if result.returncode != 0:
            return None
        output = result.stdout.strip()
        if not output:
            return None
        return yaml.safe_load(output)

    def list_configs(self) -> list[str]:
        return self._native.list_configs()
