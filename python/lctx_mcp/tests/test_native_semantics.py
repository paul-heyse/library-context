"""The native member uses the workspace condition-kernel crate."""

import pytest
from lctx_semantics import kernel_format, probe_compatible, probe_implies


def test_native_condition_boundary_uses_the_rust_kernel() -> None:
    assert kernel_format() == 1
    assert probe_compatible("truthy(a)", "!truthy(a)") is False
    assert probe_compatible("truthy(a)", "!truthy(b)") is True
    assert probe_implies("truthy(a) & truthy(b)", "truthy(a)") is True
    assert probe_implies("truthy(a)", "truthy(b)") is False
    assert probe_compatible("over_budget", "true") is None
    assert probe_compatible("truthy(a)" * 10_000, "true") is None
    with pytest.raises(ValueError):
        probe_compatible("not a condition", "true")
