from __future__ import annotations

import os
from typing import TYPE_CHECKING
from urllib.parse import urlparse

import charset_normalizer

if TYPE_CHECKING:
    from typing_extensions import ParamSpec, TypeVar

    T = TypeVar("T")
    P = ParamSpec("P")

IS_DOCKER = os.getenv("IS_DOCKER") == "1"


def is_http_url(value: str, /) -> bool:
    parsed_url = urlparse(value)
    return bool(parsed_url.scheme in {"http", "https"} and parsed_url.netloc)


def bytes_decode(value: bytes, /) -> str:
    return str(charset_normalizer.from_bytes(value)[0])
