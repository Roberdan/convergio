# convergio-gdpr

Leaf GDPR data-subject-rights handlers for Convergio.

Implemented rights:

- Article 15 access exports visible subject records.
- Article 17 erasure returns tombstones for records callers must erase.
- Article 20 portability exports portable, non-erased records as JSON.

HTTP and audit anchoring live in `convergio-server` / `convergio-durability`.
