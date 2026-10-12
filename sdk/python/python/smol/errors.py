"""Typed errors for the ``smol`` SDK.

The native extension reports errors as exceptions whose message is prefixed with
a bracketed code, e.g. ``"[KVM_UNAVAILABLE] …"``. ``wrap_native_error`` parses
that back into a typed hierarchy so callers can branch on ``err.code`` /
``isinstance``. Mirrors the Node SDK's ``errors.ts``.
"""

from __future__ import annotations

import re

__all__ = [
    "SmolError",
    "NotSupportedError",
    "InvalidConfigError",
    "ExecutionError",
    "wrap_native_error",
]


from typing import Optional


class SmolError(Exception):
    """Base error for the SDK. Carries a machine-readable ``code``."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        status: Optional[int] = None,
        problem_code: Optional[str] = None,
        retryable: bool = False,
        retry_after_s: Optional[float] = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.status = status
        """HTTP status, for an error the cloud API returned."""
        self.problem_code = problem_code
        """The server's stable problem code, e.g. ``machine_interrupted``."""
        self.retryable = retryable
        """Whether sending the same request again can succeed. A command whose
        machine went away (``MACHINE_INTERRUPTED``) is retryable, but it may have
        partly run, so the SDK leaves that retry to the caller."""
        self.retry_after_s = retry_after_s
        """How long the server asked callers to wait before retrying."""

    def __str__(self) -> str:  # pragma: no cover - trivial
        return self.message


class NotSupportedError(SmolError):
    """The active backend can't serve this operation (e.g. ``run`` on cloud)."""

    def __init__(self, message: str) -> None:
        super().__init__("NOT_SUPPORTED", message)


class InvalidConfigError(SmolError):
    """A required configuration value is missing or invalid (a usage error)."""

    def __init__(self, message: str) -> None:
        super().__init__("INVALID_CONFIG", message)


class ExecutionError(SmolError):
    """A command ran but exited non-zero (raised by ``ExecResult.assert_success``)."""

    def __init__(self, command: list[str] | str, exit_code: int, stdout: str, stderr: str) -> None:
        cmd = command if isinstance(command, str) else " ".join(command)
        super().__init__("COMMAND_FAILED", f"command exited with code {exit_code}: {cmd}")
        self.command = command
        self.exit_code = exit_code
        self.stdout = stdout
        self.stderr = stderr


_BRACKETED = re.compile(r"^\[([A-Z_]+)\]\s*(.*)$", re.DOTALL)


def wrap_native_error(err: BaseException) -> SmolError:
    """Convert any exception from the native extension into a typed ``SmolError``."""
    if isinstance(err, SmolError):
        return err
    message = str(err)
    m = _BRACKETED.match(message)
    if m:
        return SmolError(m.group(1), m.group(2))
    return SmolError("SMOLVM_ERROR", message)
