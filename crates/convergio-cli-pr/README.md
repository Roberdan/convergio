# convergio-cli-pr

Pull-request lifecycle commands for Convergio: `cvg pr stack`, `cvg pr sync`, `cvg pr merge`.

Extracted from `convergio-cli` to honour the per-crate hard cap (CONSTITUTION § Agent context budget). The CLI binary delegates `cvg pr ...` here through a thin shim — see `convergio-cli/src/commands/pr.rs`.
