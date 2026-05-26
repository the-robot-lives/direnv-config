from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Union


@dataclass(frozen=True)
class Key:
    name: str


@dataclass(frozen=True)
class Index:
    value: int


@dataclass(frozen=True)
class Wildcard:
    pass


@dataclass(frozen=True)
class Length:
    pass


Segment = Union[Key, Index, Wildcard, Length]


def parse_path(path: str) -> list[Segment]:
    if not path:
        return []

    segments: list[Segment] = []

    for token in path.split("."):
        if token == "length" and segments:
            segments.append(Length())
            continue

        bracket_pos = token.find("[")
        if bracket_pos != -1:
            key_part = token[:bracket_pos]
            if key_part:
                segments.append(Key(key_part))

            rest = token[bracket_pos:]
            while rest:
                open_pos = rest.find("[")
                if open_pos == -1:
                    break
                close_pos = rest.find("]")
                inner = rest[open_pos + 1 : close_pos]
                if inner == "*":
                    segments.append(Wildcard())
                else:
                    segments.append(Index(int(inner)))
                rest = rest[close_pos + 1 :]
        else:
            segments.append(Key(token))

    return segments


def _resolve_index(idx: int, length: int) -> int | None:
    resolved = length + idx if idx < 0 else idx
    if resolved < 0 or resolved >= length:
        return None
    return resolved


def _get_segments(current: Any, segments: list[Segment]) -> Any | None:
    if not segments:
        return current

    seg = segments[0]
    rest = segments[1:]

    if isinstance(seg, Key):
        if not isinstance(current, dict):
            return None
        child = current.get(seg.name)
        if child is None and seg.name not in current:
            return None
        return _get_segments(child, rest)

    if isinstance(seg, Index):
        if not isinstance(current, list):
            return None
        resolved = _resolve_index(seg.value, len(current))
        if resolved is None:
            return None
        return _get_segments(current[resolved], rest)

    if isinstance(seg, Wildcard):
        if not isinstance(current, list):
            return None
        collected = []
        for elem in current:
            result = _get_segments(elem, rest)
            if result is not None:
                collected.append(result)
        return collected

    if isinstance(seg, Length):
        if rest:
            return None
        if isinstance(current, (list, dict)):
            return len(current)
        return None

    return None


def get_path(root: Any, path: str) -> Any | None:
    segments = parse_path(path)
    return _get_segments(root, segments)
