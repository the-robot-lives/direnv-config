from __future__ import annotations

from pathlib import Path

from direnv_config.native import NativeBackend
from direnv_config.version import read_version

FIXTURES_DIR = Path(__file__).resolve().parent.parent.parent / "contract-tests" / "fixtures"


def test_simple_store_get_string():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    assert backend.get("cluster", "name") == "noizu"


def test_simple_store_get_nested():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    assert backend.get("cluster", "node_pool.instance_type") == "m5.xlarge"


def test_simple_store_get_integer():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    assert backend.get("cluster", "port") == 6443


def test_simple_store_get_boolean():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    assert backend.get("cluster", "enabled") is True


def test_simple_store_get_entire_config():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    data = backend.get("cluster")
    assert isinstance(data, dict)
    assert "name" in data
    assert "node_pool" in data


def test_simple_store_list_configs():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    configs = backend.list_configs()
    assert configs == ["cluster"]


def test_simple_store_version():
    version = read_version(FIXTURES_DIR / "simple-store")
    assert version == 3


def test_nested_store_array_index():
    backend = NativeBackend(FIXTURES_DIR / "nested-store")
    assert backend.get("app", "endpoints[0].host") == "api.example.com"


def test_nested_store_wildcard():
    backend = NativeBackend(FIXTURES_DIR / "nested-store")
    hosts = backend.get("app", "endpoints[*].host")
    assert hosts == ["api.example.com", "internal.example.com", "backup.example.com"]


def test_nested_store_length():
    backend = NativeBackend(FIXTURES_DIR / "nested-store")
    assert backend.get("app", "endpoints.length") == 3


def test_nested_store_chained_index():
    backend = NativeBackend(FIXTURES_DIR / "nested-store")
    assert backend.get("app", "matrix[0][1]") == 2


def test_nested_store_missing_path():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    assert backend.get("cluster", "nonexistent") is None


def test_nested_store_missing_config():
    backend = NativeBackend(FIXTURES_DIR / "simple-store")
    assert backend.get("nonexistent") is None
