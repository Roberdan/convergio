"""
Plan finite-state machine — Python reference implementation.

Deliberate duplicate of the logic in plan-fsm-rs and plan-fsm-ts.
Used by convergio-fleet cross-language fixture tests (F2-11).
"""

from enum import Enum
from typing import Optional


class PlanState(Enum):
    """States of a plan in the Convergio lifecycle."""

    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    DONE = "done"
    FAILED = "failed"


def transition(state: PlanState, event: str) -> Optional[PlanState]:
    """Attempt a state transition. Returns None for invalid transitions."""
    if state == PlanState.PENDING and event == "start":
        return PlanState.IN_PROGRESS
    if state == PlanState.IN_PROGRESS and event == "complete":
        return PlanState.DONE
    if state == PlanState.IN_PROGRESS and event == "fail":
        return PlanState.FAILED
    return None


def is_terminal(state: PlanState) -> bool:
    """Return True for terminal states where no further transition is possible."""
    return state in (PlanState.DONE, PlanState.FAILED)
