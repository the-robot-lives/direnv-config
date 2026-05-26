from __future__ import annotations

from direnv_config.path import (
    Index,
    Key,
    Length,
    Wildcard,
    get_path,
    parse_path,
)


def test_parse_simple_key():
    assert parse_path("name") == [Key("name")]


def test_parse_dotted():
    assert parse_path("a.b.c") == [Key("a"), Key("b"), Key("c")]


def test_parse_index():
    assert parse_path("items[0]") == [Key("items"), Index(0)]


def test_parse_negative_index():
    assert parse_path("items[-1]") == [Key("items"), Index(-1)]


def test_parse_wildcard():
    assert parse_path("endpoints[*].host") == [
        Key("endpoints"),
        Wildcard(),
        Key("host"),
    ]


def test_parse_length():
    assert parse_path("items.length") == [Key("items"), Length()]


def test_parse_length_first_token_is_key():
    assert parse_path("length") == [Key("length")]


def test_parse_chained_brackets():
    assert parse_path("matrix[0][1]") == [Key("matrix"), Index(0), Index(1)]


def test_parse_mixed():
    assert parse_path("folder[5].person.mobile") == [
        Key("folder"),
        Index(5),
        Key("person"),
        Key("mobile"),
    ]


def test_parse_empty():
    assert parse_path("") == []


def test_get_simple_key():
    assert get_path({"name": "alice"}, "name") == "alice"


def test_get_nested_dot():
    data = {"db": {"host": "localhost", "port": 5432}}
    assert get_path(data, "db.host") == "localhost"
    assert get_path(data, "db.port") == 5432


def test_get_missing_key():
    assert get_path({"a": 1}, "b") is None


def test_get_missing_nested_key():
    assert get_path({"a": 1}, "a.b.c") is None


def test_get_array_index():
    data = {"items": ["alpha", "beta", "gamma"]}
    assert get_path(data, "items[0]") == "alpha"
    assert get_path(data, "items[2]") == "gamma"


def test_get_negative_index():
    data = {"items": ["alpha", "beta", "gamma"]}
    assert get_path(data, "items[-1]") == "gamma"
    assert get_path(data, "items[-2]") == "beta"


def test_get_out_of_bounds():
    data = {"items": ["a"]}
    assert get_path(data, "items[5]") is None
    assert get_path(data, "items[-5]") is None


def test_get_wildcard():
    data = {
        "endpoints": [
            {"host": "a.com", "port": 80},
            {"host": "b.com", "port": 443},
        ]
    }
    assert get_path(data, "endpoints[*].host") == ["a.com", "b.com"]


def test_get_length_list():
    data = {"items": ["a", "b", "c"]}
    assert get_path(data, "items.length") == 3


def test_get_length_dict():
    data = {"m": {"a": 1, "b": 2}}
    assert get_path(data, "m.length") == 2


def test_get_chained_index():
    data = {"matrix": [[1, 2, 3], [4, 5, 6]]}
    assert get_path(data, "matrix[0][1]") == 2
    assert get_path(data, "matrix[1][-1]") == 6


def test_get_mixed_map_array():
    data = {
        "folder": [
            {"name": "zero"},
            {"name": "one"},
            {"name": "two"},
            {"name": "three"},
            {"name": "four"},
            {"person": {"mobile": "555-1234"}},
        ]
    }
    assert get_path(data, "folder[5].person.mobile") == "555-1234"


def test_get_none_value_in_dict():
    data = {"key": None}
    assert get_path(data, "key") is None
