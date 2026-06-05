# Convergio — bundle messaggi in italiano.
# Sintassi Fluent: https://projectfluent.org/fluent/guide/

# ---------- generico ----------
ok = OK
not-found = Non trovato
internal-error = Errore interno

# ---------- daemon ----------
daemon-starting = Avvio del daemon Convergio su { $url }
daemon-listening = In ascolto su { $bind }
daemon-version = Convergio { $version }

# ---------- CLI: health ----------
health-ok = Il daemon è attivo. Versione: { $version }
health-unreachable = Impossibile raggiungere il daemon su { $url }: { $reason }
health-drift = ATTENZIONE disallineamento: il workspace è alla versione { $expected }, il daemon esegue { $running }. Esegui `cvg update`.

# ---------- CLI: avviso di disallineamento pre-dispatch (P1-2) ----------
cli-drift-warning = ATTENZIONE: il CLI convergio è v{ $cli } ma il daemon su { $url } esegue v{ $daemon }
cli-drift-fix-hint = ATTENZIONE: esegui `cvg service restart` (o riavvia il daemon manualmente) per applicare le ultime modifiche.
cli-drift-suppress-hint = ATTENZIONE: per sopprimere imposta { $env }=1

# ---------- CLI: update ----------
update-rebuild-header = Ricostruzione di daemon, CLI e MCP in corso...
update-rebuild-step = compilo { $crate }
update-sync-header = Sincronizzo i binari ombreggiati
update-restart-header = Riavvio del daemon
update-restart-skipped = Riavvio saltato (--skip-restart): daemon invariato
update-verify-header = Verifica
update-no-update-needed = Nessun aggiornamento necessario: daemon già a { $version }
update-summary-ok = cvg update completato: { $prior } -> { $new } (riavviato: { $restarted })
update-step-failed = passo '{ $step }' fallito con codice { $code }
update-sync-copy-warning = Attenzione: impossibile copiare { $src } in { $dst }: { $reason }
update-release-notes-header = Note di rilascio più recenti:
update-release-notes-unavailable = Note di rilascio non disponibili (gh CLI mancante od offline).
update-changelog-header = CHANGELOG tra versione precedente e nuova:
update-changelog-empty = Nessuna voce di CHANGELOG tra la versione precedente e la nuova.
update-changelog-not-found = CHANGELOG.md non trovato; output --changelog saltato.

# ---------- CLI: status ----------
status-header = Stato Convergio
status-active-header = Piani attivi:
status-active-empty = Nessun piano attivo.
status-completed-header = Piani completati di recente:
status-completed-empty = Nessun piano completato.
status-tasks-header = Task completati di recente:
status-tasks-empty = Nessun task completato.
status-plan-line = - { $title } [{ $status }] progetto: { $project } task: { $done }/{ $total } completati
status-progress-line =   avanzamento: { $bar } { $done }/{ $total }
status-breakdown-line =   task: { $done } completati · { $submitted } inviati · { $in_progress } in corso · { $pending } in attesa · { $failed } falliti ({ $total } totali)
status-work-line =   fa: { $work }
status-next-line =   prossimi: { $tasks }
status-wave-line =     wave { $wave }: { $done } completati, { $submitted } inviati, { $in_progress } in corso, { $pending } in attesa, { $failed } falliti
status-mine-header = Filtro: solo task dell'agente { $agent }
status-task-line = - { $title } in { $plan } progetto: { $project }

# ---------- CLI: CRDT ----------
crdt-conflicts-empty = Nessun conflitto CRDT aperto.
crdt-conflicts-header = Conflitti CRDT aperti:
crdt-conflict-line = - { $entity }/{ $id } campo { $field } tipo { $type }

# ---------- CLI: ontologia ----------
ontology-branch-created = Branch ontologia creato: { $id } ({ $name })
ontology-branch-list-empty = Nessun branch ontologia.
ontology-branch-list-header = { $count } branch ontologia:
ontology-branch-list-line = - { $id } { $name } [{ $status }]
ontology-branch-transitioned = Branch ontologia { $id } passato allo stato: { $status }
ontology-entry-resolved = Voce ontologia { $key } (sorgente: { $source })
ontology-source-branch = branch
ontology-source-main = principale
ontology-source-none = nessuno
ontology-json-invalid = JSON non valido: { $value }

# ---------- CLI: workspace ----------
workspace-leases-empty = Nessun lease workspace attivo.
workspace-leases-header = Lease workspace attivi:
workspace-lease-line = - { $agent } mantiene { $kind } { $path } fino a { $expires }

# ---------- CLI: capabilities ----------
capabilities-empty = Nessuna capability locale registrata.
capabilities-header = Capability locali:
capability-line = - { $name } { $version } [{ $status }]
capability-signature-ok = Firma capability verificata per { $name } { $version } con chiave { $key }
capability-installed = Capability installata: { $name } { $version } [{ $status }]
capability-disabled = Capability disabilitata: { $name } { $version }

# ---------- CLI: setup / doctor ----------
setup-config-created = Configurazione creata: { $path }
setup-config-exists = Configurazione già presente: { $path }
setup-config-backed-up = Configurazione esistente salvata: { $path }
setup-config-repo-path-added = Aggiunto repo_path mancante in: { $path }
setup-complete = Setup completato: { $path }
setup-next-start = Prossimo passo: avvia il daemon con `convergio start`
setup-next-doctor = Poi: esegui `cvg doctor`
setup-agent-created = Snippet adapter creati per { $host }: { $path }
setup-agent-copy = Copia mcp.json nella configurazione MCP dell'agent host e prompt.txt nelle sue istruzioni.
setup-agent-claude-extras = Extra per Claude Code: copia skill-cvg-attach/ in ~/.claude/skills/cvg-attach/ e fai merge di settings.json in ~/.claude/settings.json per registrare la sessione corrente al daemon locale al SessionStart. Vedi { $path }/README.txt per i passi completi.
setup-self-check-header = Verifica di installazione Convergio (ADR-0044)
setup-self-check-ok = OK   { $name }: { $message }
setup-self-check-warn = ATTENZIONE { $name }: { $message }
setup-self-check-fail = ERRORE { $name }: { $message }
setup-self-check-summary-ok = Verifica completata con successo.
setup-self-check-summary-fail = Verifica fallita — correggi i controlli ERRORE prima di iniziare un task.
doctor-header = Diagnostica Convergio per { $url }
doctor-ok = OK { $name }: { $message }
doctor-warn = ATTENZIONE { $name }: { $message }
doctor-fail = ERRORE { $name }: { $message }
doctor-summary-ok = Diagnostica completata con successo.
doctor-summary-fail = La diagnostica ha trovato controlli falliti.
mcp-log-missing = Nessun log MCP trovato.
service-installed = File servizio scritto: { $path }
service-started = Servizio avviato.
service-stopped = Servizio fermato.
service-status-loaded = Servizio caricato.
service-status-not-loaded = Servizio non caricato.
service-uninstalled = Servizio disinstallato.

# ---------- CLI: plan ----------
plan-created = Piano creato: { $id }
plan-renamed = Piano rinominato: { $id } -> { $title }
plan-transitioned = Piano { $id } passato allo stato: { $status }
plan-not-found = Piano non trovato: { $id }
plan-list-empty = Nessun piano presente.
plan-list-header = { $count ->
    [one] Un piano:
   *[other] { $count } piani:
}
plan-list-line = #{ $number } { $title } [{ $status }]

# ---------- CLI: piano run ----------
plan-run-started = Esecuzione piano #{ $number }: { $title } ({ $pending } task in attesa)
plan-run-task-submitted = [{ $wave }.{ $seq }] { $title } → submitted ✓
plan-run-halted = Interrotto al task [{ $wave }.{ $seq }] { $title }: { $error }
plan-run-complete = Piano #{ $number } completato: { $count } task sottomessi.
plan-run-resume-hint = Riprendi con: cvg plan run { $number }
plan-run-bus-warning = ⚠️  [{ $wave }.{ $seq }] { $title }: pubblicazione sul bus del piano fallita (non bloccante): { $error }
plan-run-missing-evidence-hint = Suggerimento: a questo task mancano evidenze richieste. Allegale con `cvg evidence add { $task_id } --kind <kind> --payload <json>`, oppure avvia un agente che le produca con `cvg agent spawn --task { $task_id }`.

# ---------- CLI: triage piano ----------
plan-triage-empty = Nessun task obsoleto (pending/failed, non aggiornato da { $days } giorni).
plan-triage-header = { $count ->
    [one] Un task obsoleto (pending/failed, non aggiornato da { $days } giorni):
   *[other] { $count } task obsoleti (pending/failed, non aggiornati da { $days } giorni):
}
plan-triage-line = - [{ $status }] w{ $wave }.{ $seq } { $title } [{ $id }] (aggiornato: { $updated_at })
plan-triage-confirm = Chiudere questi { $count } task? [s/N]:
plan-triage-closed = { $count } task chiusi.
plan-triage-skipped = Triage annullato — nessun task chiuso.

# ---------- CLI: agent ----------
agent-list-empty = Nessun agente registrato.
agent-list-header = { $count ->
    [one] Un agente:
   *[other] { $count } agenti:
}
agent-list-header-active = { $count ->
    [one] Un agente attivo:
   *[other] { $count } agenti attivi:
}
agent-list-stale-hidden = { $count ->
    [one] ({ $count } agente terminato/scaduto nascosto — usa --all per mostrare)
   *[other] ({ $count } agenti terminati/scaduti nascosti — usa --all per mostrare)
}
agent-list-col-id = ID
agent-list-col-kind = TIPO
agent-list-col-status = STATO
agent-list-col-current-task = TASK CORRENTE
agent-list-col-task = TASK
agent-list-col-branch = BRANCH
agent-list-col-last-hb = ULT_HB
agent-list-col-claimed = ASSEGN
agent-list-col-last-topic = ULT_TOPIC
agent-list-col-capabilities = CAPACITÀ
agent-list-col-leases = LEASE
agent-list-col-last-audit = ULT_AUDIT
agent-show-header = Agente { $id }:
agent-show-kind = Tipo
agent-show-status = Stato
agent-show-registered = registrato { $at }
agent-show-capabilities = Capacità
agent-show-last-topic = Ultimo topic bus
agent-show-no-last-topic = nessuna attività bus
agent-show-claimed-tasks = Task assegnati
agent-show-no-claimed-tasks = nessun task assegnato
agent-show-current-task = Task corrente
agent-show-no-current-task = nessun task corrente
agent-show-plan = Piano
agent-show-task-status = Stato
agent-show-leases = Lease workspace attivi
agent-show-no-leases = nessun lease
agent-show-recent-audit = Audit recente
agent-show-no-recent-audit = nessun audit recente
agent-show-recent-prs = PR recenti per questo agente
agent-show-no-recent-prs = nessuna PR recente
agent-retire-stale-summary = { $count ->
    [one] Ritirato { $count } agente scaduto (soglia { $threshold_min } min):
   *[other] Ritirati { $count } agenti scaduti (soglia { $threshold_min } min):
}
agent-retire-stale-dry-run = { $count ->
    [one] Ritirerei { $count } agente scaduto (dry-run, soglia { $threshold_min } min):
   *[other] Ritirerei { $count } agenti scaduti (dry-run, soglia { $threshold_min } min):
}
agent-retire-stale-none = nessun agente scaduto sotto la soglia
agent-retire-success = Agente { $id } ritirato
agent-retire-not-found = Agente non trovato: { $id } (già ritirato o mai registrato)
agent-retire-help-after-422 = L'heartbeat non può impostare status='retired' — usa `cvg agent retire { $id }` (oppure POST /v1/agent-registry/agents/{ $id }/retire).
agent-not-found = Agente non trovato: { $id }

# ---------- rifiuti dei gate (lato umano) ----------
# Il campo `code` resta in inglese (è contratto API).
# Il `message` è ciò che l'umano legge.
gate-refused-evidence = Evidenze mancanti: { $kinds }
gate-refused-no-debt = Debito tecnico trovato nelle evidenze: { $markers }
gate-refused-no-stub = Marker di scaffolding trovati nelle evidenze: { $markers }
gate-refused-zero-warnings = Il segnale di build/lint non è pulito: { $signals }
gate-refused-plan-status = Il piano è { $status }; nuove transizioni non accettate
gate-refused-wave-sequence = { $count ->
    [one] Un task della wave precedente è ancora aperto
   *[other] { $count } task delle wave precedenti sono ancora aperti
}

# ---------- audit ----------
audit-clean = Catena audit verificata: { $count } eventi, nessuna manomissione rilevata.
audit-broken = Catena audit rotta alla sequenza { $seq }.
audit-compensate-dry-run = Dry-run: compenserei l'evento audit { $seq } ({ $transition }).
audit-compensate-applied = Compensazione applicata per l'evento audit { $seq } ({ $transition }).
audit-compensate-apply-hint = Applica con: cvg audit compensate { $seq } --apply
audit-compensate-action = Azione compensante:
{ $action }

# ---------- CLI: actions ----------

# ---------- CLI: pubblico ----------
public-algorithms-generated = Registro algoritmi generato per il tenant { $tenant } ({ $count } voci): { $path }

actions-list-empty = Nessuna azione trovata.
actions-list-header = { $count ->
    [one] Un'azione:
   *[other] { $count } azioni:
}
actions-list-line = - [{ $capability }] { $name } — { $summary }

# ---------- CLI: gates ----------
gates-list-empty = Nessuna precondizione gate trovata.
gates-list-header = { $count ->
    [one] Un gate:
   *[other] { $count } gate:
}
gates-list-line = - { $gate } active={ $active } reads={ $reads } refusals={ $refusals } evidence_required={ $evidence_required }

# ---------- CLI: pr stack ----------
pr-stack-empty = Nessuna PR aperta.
pr-stack-header = { $count ->
    [one] Una PR aperta:
   *[other] { $count } PR aperte:
}
pr-stack-no-manifest = manifest Files-touched assente
pr-stack-manifest-mismatch = il manifest non corrisponde al diff
pr-stack-manifest-unverified = manifest non verificato (recupero diff gh fallito)
pr-stack-files-summary = { $count ->
    [one] un file
   *[other] { $count } file
}
pr-stack-suggested-order = Ordine di merge suggerito:

# ---------- CLI: session resume ----------
session-resume-header = Riavvio sessione Convergio
session-resume-health-ok = Daemon: ok (versione { $version })
session-resume-health-down = Daemon: NON attivo (versione { $version })
session-resume-audit-ok = Catena audit: ok ({ $count } eventi)
session-resume-audit-broken = Catena audit: ROTTA ({ $count } eventi verificati)
session-resume-plan-line = Piano: { $title } [{ $status }] progetto: { $project } id: { $id }
session-resume-counts-line = Task: { $done }/{ $total } completati — in corso: { $in_progress }, in revisione: { $submitted }, da fare: { $pending }
session-resume-next-empty = Prossima priorità: nessuna (nessun task aperto).
session-resume-next-header = Prossima priorità (primi task aperti):
session-resume-next-line =   - w{ $wave }.{ $sequence } { $title } [{ $id }]
session-resume-prs-empty = PR aperte: nessuna.
session-resume-prs-unavailable = PR aperte: gh non disponibile (saltato).
session-resume-prs-header = PR aperte:
session-resume-pr-line =   - #{ $number } { $title } ({ $branch })
session-resume-pr-line-draft =   - #{ $number } [bozza] { $title } ({ $branch })
session-resume-pack-line = Context-pack del task { $task_id }: { $nodes } nodi, { $files } file, ~{ $est_tokens } token

# ---------- CLI: session register-and-poll ----------
session-register-poll-header = Convergio sessione register-and-poll
session-register-poll-registered = Registrato come: { $id } (kind={ $kind }, host={ $host })
session-register-poll-heartbeat = Heartbeat: { $status }
session-register-poll-plans-header = { $count ->
    [0] Piani attivi: nessuno
    [one] Piani attivi (1):
   *[other] Piani attivi ({ $count }):
}
session-register-poll-plan-line =   - { $id } { $title }
session-register-poll-direct-header = { $count ->
    [0] Messaggi diretti in attesa: nessuno
    [one] Messaggi diretti in attesa (1):
   *[other] Messaggi diretti in attesa ({ $count }):
}
session-register-poll-announcements-header = { $count ->
    [0] Annunci di piano in attesa: nessuno
    [one] Annunci di piano in attesa (1):
   *[other] Annunci di piano in attesa ({ $count }):
}
session-register-poll-message-line =   - piano { $plan } seq { $seq } [{ $topic }] sender={ $sender }
session-register-poll-message-line-consumed =   - piano { $plan } seq { $seq } [{ $topic }] sender={ $sender } (consumato)

# ---------- CLI: session pre-stop ----------
session-pre-stop-header = Rapporto pre-arresto (agent_id={ $agent_id }, force={ $force })
session-pre-stop-mark-pass = ok
session-pre-stop-mark-fail = FAIL
session-pre-stop-mark-todo = da fare
session-pre-stop-check-line =   [{ $mark }] { $id } — { $label }
session-pre-stop-finding-line =         - { $finding }
session-pre-stop-todo-line =         pianificato nel task { $task_id }

# ---------- brand (CLI: about) ----------
# I marchi (claim/subline/nome prodotto) NON vengono tradotti — sono
# trade dress e vivono in `convergio-brand`. Queste chiavi sono le
# etichette che circondano il marchio quando la CLI si presenta.
brand-about-tagline = Convergio — { $version }
brand-about-source = Sorgente: { $url }
brand-about-help = Digita `cvg --help` per iniziare.

# ---------- CLI: coherence routes ----------
coherence-routes-summary = Verificate { $code } route nel codice contro { $docs } route documentate; { $violations } divergenza/e.
coherence-routes-ok = Coerenza route: ok (nessuna divergenza).
coherence-routes-header = Coerenza route: { $count } divergenza/e:
coherence-routes-missing-in-docs = missing_in_docs: { $method } { $path } (nel codice in { $file }, non documentata)
coherence-routes-missing-in-code = missing_in_code: { $method } { $path } (documentata in { $file }, assente nel codice)
coherence-routes-method-mismatch = method_mismatch: { $path } — codice ha [{ $code_methods }], docs hanno [{ $doc_methods }]

# ---------- CLI: coherence adrs ----------
coherence-adrs-summary = Verificate { $checked } ADR, { $findings } rilievo/i.
coherence-adrs-empty = Coerenza ADR: ok (nessuna deriva di stato rilevata).
coherence-adrs-table-header = ADR    Dichiarato                       Rilievo                      Evidenza
coherence-adrs-finding-accepted-no-evidence = accettata, nessuna evidenza
coherence-adrs-finding-proposed-likely-shipped = proposta, probabilmente rilasciata
coherence-adrs-finding-broken-supersession = supersessione rotta

# ---------- CLI: coherence agents ----------
coherence-agents-summary = Verificate { $checked } PR mergiate in [{ $since }], { $findings } rilievo/i; strict_passes={ $strict }.
coherence-agents-empty = Coerenza agenti: nessuna PR mergiata nella finestra.
coherence-agents-table-header = PR     Autore                 Agente associato         Rilievo                    Evidenza
coherence-agents-finding-no-registered-agent = no_registered_agent
coherence-agents-finding-no-heartbeat = no_heartbeat_in_window
coherence-agents-finding-no-coordination = no_coordination
coherence-agents-finding-clean = pulito

# ---------- CLI: coherence handshake (F1) ----------
coherence-handshake-summary = cvg coherence handshake — daemon: { $daemon } (timeout { $timeout }ms)
coherence-handshake-phase-1 = registrazione A+B
coherence-handshake-phase-2 = A → ping
coherence-handshake-phase-3 = B riceve e risponde con pong
coherence-handshake-phase-4 = A riceve pong
coherence-handshake-phase-5 = ack
coherence-handshake-phase-6 = ritiro
coherence-handshake-success = handshake completato in { $elapsed }ms (timeout era { $timeout }ms)
coherence-handshake-fail = handshake fallito dopo { $elapsed }ms (timeout era { $timeout }ms)
coherence-handshake-timeout = handshake scaduto dopo { $elapsed }ms (deadline { $timeout }ms)

# ---------- CLI: coherence plan-execution (ADR-0044) ----------
coherence-plan-execution-summary = Piano { $plan }… — { $closed } task chiusi, { $compliant } conformi, punteggio { $score }%
coherence-plan-execution-plan-checks = Livello piano: registry={ $registry }  bus={ $bus }
coherence-plan-execution-task-ok = OK   { $id }… { $title }
coherence-plan-execution-task-fail = ERRORE { $id }… { $title } — mancanti: { $missing }

# ---------- CLI: coherence close-post-hoc (ADR-0026, retro H5) ----------
coherence-close-post-hoc-header = cvg coherence close-post-hoc — { $total } chiusura/e dal { $since }
coherence-close-post-hoc-clean = nessuna riga close-post-hoc nella finestra — pulito.
coherence-close-post-hoc-by-agent = per agente:
coherence-close-post-hoc-by-plan = per piano:
coherence-close-post-hoc-rows = righe:
coherence-close-post-hoc-row-reason = motivo: { $reason }

# ---------- CLI: coherence fleet (issue #177) ----------
coherence-fleet-header = cvg coherence fleet — { $repos } repo in { $path }
coherence-fleet-clean = nessun rilievo — pulito.
coherence-fleet-findings = { $count } rilievo/i:

# ---------- CLI: bus tail / list (P1.2) ----------
bus-tail-following = In ascolto sul bus del piano { $plan } (Ctrl-C per uscire)
bus-tail-disconnect = stream del bus disconnesso, riconnessione in corso...
bus-tail-streaming-unavailable-fallback-polling = ATTENZIONE: il daemon non espone lo streaming; passo al polling ogni 1s.
bus-tail-empty = Nessun messaggio.
bus-list-summary = Piano { $plan } — { $count } messaggio/i

# ---------- CLI: discover (F2) ----------
discover-header = Convergio scoperta peer (al { $at })
discover-active-peers = PEER ATTIVI (heartbeat negli ultimi { $since }, stato != terminato/scaduto):
discover-recent-bus = ATTIVITÀ BUS RECENTE (top 5 topic, ultima ora):
discover-your-plans = I TUOI PIANI (dove appare il tuo agent_id, più recenti prima):
discover-empty-peers = (nessun peer attivo nella finestra)
discover-empty-bus = (nessuna attività recente sul bus)
discover-empty-plans = (nessun piano assegnato)

# ---------- CLI: task complete orchestrator (P1-1) ----------
task-complete-step-graph = [complete] graph for-task --semantic …
task-complete-step-embed = [complete] embed for-task …
task-complete-step-evidence-graph = [complete] aggiunta evidenza graph_pack …
task-complete-step-evidence-embed = [complete] aggiunta evidenza embed_query …
task-complete-step-evidence-pr = [complete] aggiunta evidenza pr_link (PR #{ $pr }) …
task-complete-step-submit = [complete] transizione → submitted …
task-complete-step-thor = [complete] validazione piano (Thor) …
task-complete-thor-failed = Validazione Thor fallita: { $verdict }

# ---------- CLI: cvg pr link / who / merge / sync (follow-up i18n P5) ----------
pr-link-success = PR #{ $pr } collegata al piano { $plan } (repo: { $repo })
pr-who-empty = Nessuna proprieta PR registrata per { $repo }#{ $pr }
pr-who-ownership = { $repo }#{ $pr } -> agente={ $agent } piano={ $plan } task={ $task }
pr-who-more = (mostrati gli ultimi { $count } collegamenti)
pr-merge-header = cvg pr merge -- PR #{ $pr } (ramo { $head })
pr-merge-refused = rifiutato: il controllo 4-check non e' passato.
pr-merge-tracked-header = task tracciati aggiornati ({ $count }):
pr-merge-failed-evidence-header = scritture di evidence fallite ({ $count }): merge_record NON e' stato allegato
pr-merge-note-prefix = nota:
pr-sync-header = cvg pr sync -- analizzate { $scanned } PR mergiate, trovate { $tracked } coppie (PR, task)
pr-sync-transitioned-header = transizionate ({ $count }): pending -> submitted
pr-sync-transitioned-empty = transizionate (0): nessun task -> submitted
pr-sync-skipped-header = saltate ({ $count }): gia' submitted o done
pr-sync-failed-header = fallite ({ $count }): rifiuto del gate o errore di trasporto
pr-sync-link-failures-header = errori_link ({ $count }): POST /pr-links rifiutato
