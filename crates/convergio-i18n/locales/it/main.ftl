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
cli-version-drift = avviso: CLI { $cli } diverge dal daemon { $daemon } — esegui `cvg update` per sincronizzare

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
agent-list-col-capabilities = CAPACITÀ
agent-list-col-leases = LEASE
agent-list-col-last-audit = ULT_AUDIT
agent-show-header = Agente { $id }:
agent-show-kind = Tipo
agent-show-status = Stato
agent-show-registered = registrato { $at }
agent-show-capabilities = Capacità
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

# ---------- CLI: pr stack ----------
pr-stack-empty = Nessuna PR aperta.
pr-stack-header = { $count ->
    [one] Una PR aperta:
   *[other] { $count } PR aperte:
}
pr-stack-no-manifest = manifest Files-touched assente
pr-stack-manifest-mismatch = il manifest non corrisponde al diff
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
