import os
from pathlib import Path

import pytest


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption("--kgw-checkout", action="store", default=os.getenv("WATERLARP_KGW_CHECKOUT"))


@pytest.fixture
def kgw_checkout(request: pytest.FixtureRequest) -> Path | None:
    value = request.config.getoption("--kgw-checkout")
    return None if value is None else Path(value)
