/**
 * Plan finite-state machine — TypeScript reference implementation.
 *
 * Deliberate duplicate of the logic in `plan-fsm-rs` and `plan-fsm-py`.
 * Used by convergio-fleet cross-language fixture tests (F2-11).
 */

/** States of a plan in the Convergio lifecycle. */
export type PlanState = 'pending' | 'in_progress' | 'done' | 'failed';

/** Attempt a state transition. Returns `null` for invalid transitions. */
export function transition(state: PlanState, event: string): PlanState | null {
  if (state === 'pending' && event === 'start') return 'in_progress';
  if (state === 'in_progress' && event === 'complete') return 'done';
  if (state === 'in_progress' && event === 'fail') return 'failed';
  return null;
}

/** Returns `true` for terminal states where no further transition is possible. */
export function isTerminal(state: PlanState): boolean {
  return state === 'done' || state === 'failed';
}
