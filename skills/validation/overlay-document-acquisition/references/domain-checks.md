# Document Acquisition Overlay Domain Checks

This reference defines the domain checks used by `validation/overlay-document-acquisition`.

| Check | Validator Question | Reject When |
| --- | --- | --- |
| `document_type_identification` | Are acquired files classified as the expected procurement document types using content and notice evidence? | The claimed type conflicts with content probes, notice context, or required attachment metadata. |
| `download_completeness` | Were all required attachments downloaded or explicitly recorded as unavailable with evidence? | A required attachment is missing without source-changed, unavailable, or retry-exhausted evidence. |
| `content_type_verification` | Do MIME type, extension, and lightweight content observations agree enough for safe parsing? | MIME, extension, or content probes conflict in a way that blocks trustworthy parsing. |
| `file_size_plausibility` | Are byte counts plausible for the claimed document type and acquisition status? | A file is empty, truncated, unexpectedly huge, or otherwise implausible without rationale. |
| `document_set_hash_consistency` | Are document hashes and set summaries stable across manifest and daemon inventory evidence? | Manifest hash summaries conflict with daemon inventory content hashes. |

The overlay may flag weird-valid cases with partial content-type or file-size evidence, but it must not reject them unless one of the listed reject conditions is present.
