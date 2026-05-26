# Post-mortem — 2026-05-08 — "il claude dentro convergio non risponde più"

**Author:** investigated by Claude (claude-opus-4-7) on 2026-05-08, ~23:30 CEST
**Status:** root cause identified, preventive plan proposed (NOT yet implemented)
**Severity:** dev-loop blocker — convergio is wedged but no production impact

## TL;DR

Il sistema sembrava "non rispondere" perché **10 processi `copilot:gpt-5.2` lanciati dal dispatcher sono bloccati in un loop infinito di `gate_refused`**. Bruciano CPU e chiamate API senza avanzare. Il vero claude (l'utente) non c'entra: non c'erano processi `claude` runner in giro al momento dell'incidente. La dirtiness contorno (worktree `.broken-*`, tmp dir orfane, log corrotti, 23 GB di worktree, 23k righe `agent_processes`, 42k audit) è il prodotto cumulativo di **bug strutturali nel layer 4 e nel layer 1**, non di un singolo evento.

## Stato osservato (2026-05-08 23:00–23:30 CEST)

### Processi vivi

| PID | Comando | Stato |
|-----|---------|-------|
| 93065 | `convergio start` (daemon) | OK, listener su :8420 |
| 1553–1645 (10 PID) | `copilot:gpt-5.2` runner agents | **vivi ma in loop gate_refused** |
| 59186 | `convergio-mcp --url http://127.0.0.1:8420` | OK, MCP bridge stdio |
| 92913 | `claude --dangerously-skip-permissions --remote-control` | **questa è la sessione operatore corrente — NON è un agente convergio** |
| 92270 | `git worktree remove --force agent-0e1c415` | **ERA bloccato per ~1 min, ora terminato** |
| 57827 | `tail -f ...agent-c619274.../blc702eba.output` | zombie di sessione 22:49 |

### File system

| Path | Size | Note |
|------|------|------|
| `~/.convergio/` | **1.6 GB** | logs + DB + modelli |
| `~/.convergio/v3/state.db` | 100 MB + 14 MB WAL | mai checkpointed; 42 781 righe `audit_log` |
| `~/GitHub/convergio/.claude/worktrees/` | **23 GB** | 30 worktree attivi + 1 `.broken-*` orfano |
| `~/.convergio/convergio.err.log` | 246 KB | **3 290 occorrenze** di `Error: Os { code: 48, kind: AddrInUse }` |
| `~/.convergio/v3/daemon.log` | 16 KB, **fermo dal 2026-05-02** | log path superato — ora si scrive in `~/.convergio/daemon.log` |
| `~/.convergio/mcp.log` | 81 KB | 246 `gate_refused` su 264 `submit_task` (**93 % failure**) + 3 righe corrotte da write race |
| `/tmp/claude-502/` | 272 KB | 3 directory orfane (`agent-0e1c415`, `agent-c619274`, `agent-ea3ceac`) per worktree già rimossi |

### Stato DB (`~/.convergio/v3/state.db`)

```
tasks:           307 done | 8 failed | 60 pending | 24 submitted
plans:            16 completed | 25 cancelled | 13 active | 15 draft
agents:           62 terminated | 0 attivi
agent_processes: 23 152 exited | 10 running (i 10 copilot in loop)
audit_log:       42 781 righe (mai prunato)
```

I 60 pending più vecchi risalgono al 2026-05-01. Il loro `updated_at` viene aggiornato a ogni reaper tick (`2026-05-08T21:09:56.027*` per quasi tutti) ma lo `status` non avanza. Il reaper "tocca" e basta.

## Cosa è successo davvero

### 1. Il loop infinito `gate_refused` (causa dello "stallo")

```
mcp.log (estratto)
{"action":"submit_task","code":"gate_refused","next":"fix_add_evidence_retry_submit", ...}
{"action":"explain_last_refusal","code":"ok",     "next":"fix_add_evidence_retry_submit", ...}
{"action":"submit_task","code":"gate_refused","next":"fix_add_evidence_retry_submit", ...}
{"action":"explain_last_refusal","code":"ok",     "next":"fix_add_evidence_retry_submit", ...}
… (ripetuto 246 volte)
```

I 10 runner `copilot:gpt-5.2` (PID 1553–1645, tutti spawnati nel giro di 1.5 secondi alle 21:12:51–53) chiamano l'API in questa sequenza, ogni volta:

1. `submit_task` → daemon risponde HTTP 409 `gate_refused` (`crates/convergio-server/src/error.rs:126`, `crates/convergio-durability/src/facade_transitions.rs:48`)
2. `explain_last_refusal` → daemon spiega cosa manca (`fix_add_evidence_retry_submit`)
3. Goto 1

Non c'è nessuno **stato di backoff o rinuncia** lato runner: il prompt template istruisce l'agente a "fix evidence and retry" all'infinito. Il `MaxParallel` (env `CONVERGIO_EXECUTOR_MAX_PARALLEL`) limita la concorrenza ma NON impedisce a un task in `submitted` di essere rilavorato di nuovo dopo che il reaper lo ribalta in `pending`.

### 2. Il rename `.broken-20260508-165631`

La dir `agent-ac4e105.broken-20260508-165631` esiste come **directory di file senza `.git`**, sganciata da git. Stesso `gitdir` registrato in `.git/worktrees/agent-ac4e105/` puntato dal *nuovo* worktree (timestamp 16:56:31 = stesso minuto del commit `15774b8 test(executor): make dispatch tests env-independent`).

**Origine:** non c'è nessun codice in `crates/convergio-executor/src/worktree.rs` o altrove nel repo che faccia `mv → .broken-`. Né lo zsh history mostra un comando manuale. È un'azione **esterna a convergio**: probabilmente uno script di pulizia/setup ad-hoc dell'utente o un hook di un altro tool. Va trovato e neutralizzato (vedi prevenzione P5).

### 3. `git worktree remove --force` impiccato (~1 minuto)

PID 92270 è rimasto al 30–45 % CPU per circa un minuto su `agent-0e1c415`. Si è risolto da solo. Ipotesi: file handle aperto da una vecchia sessione claude in `/tmp/claude-502/-…-agent-0e1c415-…/` (4 sotto-sessioni con ~7 dirs `.output`) → `git worktree remove --force` su macOS aspetta che i descriptor si chiudano. Confermato dal fatto che la dir tmp è ancora lì, orfana, dopo la rimozione del worktree.

`crates/convergio-executor/src/worktree.rs:92-101` esegue il remove come "best-effort, errors swallowed". Quando si blocca, **non c'è timeout**, e il chiamante (heartbeat o cleanup hook) si appende sopra.

### 4. Log path drift

Esistono **due** `daemon.log`:
- `~/.convergio/v3/daemon.log` — fermo al 2026-05-02 (codice vecchio)
- `~/.convergio/daemon.log` — attivo, 2.2 MB, 27 restart del daemon registrati

Il codice corrente scrive in `~/.convergio/`, ma `~/.convergio/v3/daemon.log` non è stato cancellato → confusione operatore.

### 5. Concorrenza nei log MCP

In `mcp.log` ci sono 3 righe corrotte tipo:
```
{"{action""action:""list_tasks:"",list_agents""code,"":code""ok:"",ok""next,""next:"null:,null",ok""ok:"true:,true",ts""ts:"1778273759:}1778273759
```

Due processi MCP scrivono **interleaved** sullo stesso file senza `O_APPEND` o lock. Su POSIX, `write(2)` di un append < PIPE_BUF (4 KB) è atomico solo se il file è aperto con `O_APPEND`. Il bridge non lo fa.

### 6. Nessuna idempotenza all'avvio

`crates/convergio-server/src/main.rs` chiama direttamente `axum::Server::bind(...)`. Se la porta è già occupata, fallisce con `AddrInUse` e termina. **Nessun controllo "daemon già attivo"**, **nessuno script wrapper che salta start se il daemon risponde** → 3 290 errori `AddrInUse` in `convergio.err.log` da retry continui (probabilmente CLI subcommands che si aspettano "ensure daemon running" senza prima testare `GET /v1/health`).

### 7. Reaper che reapa ma non fa avanzare

Daemon log mostra ticks con `reaped=24` per ore consecutive: lo stesso set di 24 task `submitted` viene "reaped" ma il loop `gate_refused` lato runner li riporta indietro in `submitted` istantaneamente. È un **conflitto reaper ↔ runner senza stato terminale**.

### 8. Nessun cleanup di `/tmp/claude-502/`

Le dir di sessione di Claude Code (`/tmp/claude-502/<worktree-encoded>/<uuid>/`) restano dopo che il worktree viene rimosso da convergio. Tre dir orfane confermate (`agent-0e1c415`, `agent-c619274`, `agent-ea3ceac`).

### 9. Esplosione dei worktree (23 GB)

30 worktree attivi, ognuno con il proprio target dir Rust, lockfile separati, fastembed model copiato (BGE-M3 ≈ 380 MB). La pulizia post-merge non è automatica: `agent/4aa5628` ha branch già mergiato in main ma worktree presente. Servono `git worktree prune` + rimozione esplicita.

## Cause-radice (root causes)

| # | Categoria | Bug |
|---|-----------|-----|
| **R1** | runner protocol | I runner non hanno backoff/rinuncia su `gate_refused` ripetuti — loop infinito guidato dal prompt template. |
| **R2** | dispatcher | `Executor::tick` non distingue task con N rifiuti consecutivi: ridispaccia per sempre. |
| **R3** | reaper | "reapa" task `submitted` ma non li sposta in stato terminale → conflitto con runner che li ri-`submit`. |
| **R4** | worktree mgmt | `worktree::prepare()` riusa una dir esistente senza verificare che sia un worktree git valido (`worktree.rs:57-61`). |
| **R5** | worktree mgmt | `worktree::cleanup()` chiama `git worktree remove --force` senza timeout né forzato unmount dei file handle. |
| **R6** | log/IPC | `convergio-mcp` scrive log senza `O_APPEND` → write race a due processi corrompe JSONL. |
| **R7** | startup | Nessun check di "daemon già attivo": `Server::bind` muore su `AddrInUse`, CLI ritenta in loop. |
| **R8** | observability | Path log `~/.convergio/v3/daemon.log` non più scritto ma mai rimosso → operatore guarda log obsoleto. |
| **R9** | gc | `agent_processes`, `audit_log`, `/tmp/claude-502/`, worktree mergiati: nessun GC schedulato. Crescita lineare illimitata. |
| **R10** | misterious | Origine sconosciuta del rinominio `.broken-TIMESTAMP` — esterno a convergio. |

## Piano di prevenzione

### Immediato (≤ 1 PR ciascuno, alta priorità)

**P1 — runner backoff su `gate_refused` ripetuti (chiude R1, R2, R3).**
In `crates/convergio-runner/` (o nel template MCP): dopo N (default 3) `gate_refused` consecutivi sullo stesso `task_id` con stessa firma di rifiuto, il runner deve:
- chiamare `cvg task transition <id> failed` con `reason: gate_loop`
- liberare il claim (capi-cassetto via `submitted → failed` con audit)
- exit non-zero
Il dispatcher deve **non** ridispacciare task già passati per `failed` con `reason=gate_loop` (gate `failure_quarantine`).

**P2 — `worktree::prepare()` valida prima di riusare (chiude R4).**
File `crates/convergio-executor/src/worktree.rs`. Cambiare la branch `if path.exists()` (line 57) in:
```rust
if path.exists() {
    if is_valid_worktree(repo_root, &path)? { return Ok(path); }
    quarantine(&path)?;   // mv path → path.<ts>.invalid, log warn
}
```
Test: aggiungere `quarantines_invalid_dir_before_create` in `crates/convergio-executor/src/worktree.rs`.

**P3 — `cleanup()` con timeout + lsof drain (chiude R5).**
Stesso file. Wrappare `run_git(["worktree","remove",...])` in `Command::new(...).timeout(30s)` (via `wait_timeout` crate o `tokio::process` async). Su timeout: log error, non bloccare il tick. In più, prima di remove, eseguire `git worktree prune` come idempotenza per gli orfani.

**P4 — daemon idempotente all'avvio (chiude R7).**
`crates/convergio-server/src/main.rs`: prima di `bind()`, fare `TcpStream::connect("127.0.0.1:8420")`. Se risponde a `GET /v1/health` con il proprio fingerprint (uuid in `~/.convergio/run.lock`), exit 0 con messaggio "daemon already running". Aggiungere a `convergio start` un'option `--ensure` che fa esattamente questo. Sopprimerà l'AddrInUse spam.

**P5 — trovare chi rinomina in `.broken-*` (chiude R10).**
Aggiungere un `fs_events` watcher temporaneo in dev (1 settimana) su `.claude/worktrees/` che logga ogni `RENAMED` event con il PID del processo chiamante (su macOS via `fs_usage` o `eslogger`). Quando l'evento si ripresenta, abbiamo il colpevole. In alternativa: verificare `~/.local/share/claude/`, hooks di gstack, e setup-fleet scripts dell'utente.

### Hardening (settimana prossima)

**P6 — write atomic per `mcp.log` (chiude R6).**
`crates/convergio-mcp/src/...`: aprire il log con `OpenOptions::new().append(true).create(true)` invece di scrivere via `std::fs::write` o write+seek. Un singolo `write_all` di una riga `<` PIPE_BUF è atomico in O_APPEND.

**P7 — log path unico documentato (chiude R8).**
Eliminare `~/.convergio/v3/daemon.log` (è obsoleto). Aggiungere `cvg doctor` check che warning se rilevato. Documentare la convenzione in `AGENTS.md` § "Background loops".

**P8 — GC schedulato (chiude R9).**
Estendere il reaper: ogni N tick (configurable, default 1 ora):
- `DELETE FROM agent_processes WHERE status='exited' AND ended_at < now()-7d`
- `DELETE FROM audit_log WHERE seq < (max(seq) - keep_window)` (default keep 50 000 righe; spec da scrivere)
- `git -C <repo> worktree prune` + `find .claude/worktrees -maxdepth 1 -type d -empty -delete`
- `find /tmp/claude-502 -maxdepth 1 -type d -mtime +1 -name '*-agent-*' -delete` (scope-locked)
Aggiungere ADR per la retention policy.

**P9 — auto-merge ↔ worktree cleanup (sotto-task di R9).**
Hook `post-merge` o `cvg pr land`: dopo il merge in main, chiamare `worktree::cleanup(&repo, &task_id)` per il branch agente mergiato. Test: `make_pr_lands_then_worktree_gone`.

### Operativo (subito, manuale, non richiede PR)

**M1 — terminare i 10 copilot in loop.**
```bash
for pid in 1553 1562 1572 1573 1583 1592 1606 1618 1627 1645; do kill "$pid"; done
sleep 5
# il watcher li marca exited entro 30s
```

**M2 — pulizia orfani sicura.**
```bash
rm -rf ~/GitHub/convergio/.claude/worktrees/agent-ac4e105.broken-20260508-165631
rm -rf /tmp/claude-502/-Users-Roberdan-GitHub-convergio--claude-worktrees-agent-{0e1c415,c619274,ea3ceac}-*
rm   ~/.convergio/v3/daemon.log   # log path obsoleto
```

**M3 — checkpoint del WAL e reindex.**
```bash
sqlite3 ~/.convergio/v3/state.db 'PRAGMA wal_checkpoint(TRUNCATE); VACUUM;'
```
Riduce i 100 MB + 14 MB di WAL a stato compatto.

**M4 — flush dei task pending vecchi (richiede l'okay).**
Spostare i 60 pending più vecchi di 48 h in `cancelled` con audit:
```bash
sqlite3 ~/.convergio/v3/state.db <<'SQL'
UPDATE tasks
SET status = 'failed', updated_at = datetime('now')
WHERE status = 'pending' AND created_at < datetime('now', '-2 days');
SQL
```
Decisione operativa, non automatizzabile finché non chiariamo se erano work-in-progress reale.

## Verifica preventiva

Dopo P1+P2+P3+P4 implementati e i passi M1–M3 eseguiti, il sistema deve passare questo test di salute (da aggiungere come `cvg doctor`):

- [ ] `convergio start && convergio start` → secondo invoco ritorna 0 con "already running"
- [ ] Un task con evidence non valida → runner termina dopo 3 `gate_refused`, task in `failed`, no spam in `mcp.log`
- [ ] Worktree `.broken-` o invalid → quarantinato + nuovo creato, no `git worktree remove` impiccato
- [ ] Dopo 24h idle → `agent_processes` exited rows < 1000, `audit_log` < 100k righe, `/tmp/claude-502/` < 10 MB
- [ ] `mcp.log` → 0 righe corrotte dopo 1 ora di carico parallelo a 4 runner

## Allegati / dati raccolti

- `mcp.log` count actions: 264 submit_task, 246 gate_refused, 223 explain_last_refusal, 82 add_evidence
- `convergio.err.log`: 3 290 × `AddrInUse`
- `daemon.log`: 27 daemon restarts dal 2026-05-02 al 2026-05-08
- DB tasks pending oldest: `c6192749…` creato 2026-05-01 (7 giorni di pending)
- 30 worktree attivi, 1 `.broken-*`, 23 GB totali su `.claude/worktrees/`
- 10 PID copilot vivi, tutti spawnati 2026-05-08T21:12:51–53 entro 1.4 s

— end of post-mortem —
