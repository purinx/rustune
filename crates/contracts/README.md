# Wire boundary

`contracts` contains serializable API values only. It does not depend on domain,
SQL, HTTP or browser APIs. The server explicitly projects domain getters into DTOs
and validates deserialized requests before changing state. Domain IDs convert via
`as_uuid()` and `TrackId::new`; no random UUID generation occurs in either crate.
Time strings are UTC RFC 3339, durations milliseconds, sizes bytes, revisions u64.
The server supplies formatted timestamps and relative source paths; database rows
and raw parser tags are never serialized. No purchaser, account or managed-media
path field exists. Only settings may expose the source root's absolute path.

`MetadataPatch` preserves omitted / value / null as Keep / Set / Clear.
Clear removes the manual value and re-resolves metadata. Empty strings fail
validation. Call `validate()` after JSON decoding; syntactically valid JSON alone
does not establish domain validity. Settings also preserve the three wire states;
their non-nullable values reject Clear during validation instead of silently
replacing invalid values with defaults.

DTO fields remain plain values so that responses and future unknown error codes
can be decoded without domain dependencies. An unknown error code must use a
generic UI fallback. Public errors are built from `ErrorCode`'s fixed Japanese
messages, never raw OS or parser messages. HTTP status mapping is server-owned.
