# Procurement Ingestion Overlay Domain Checks

This reference defines the domain checks used by `validation/overlay-procurement-ingestion`.

| Check | Validator Question | Reject When |
| --- | --- | --- |
| `notice_completeness` | Are notice identity, buyer, title, dates, value, and procurement route present or explicitly explained? | Required identity fields are absent without rationale. |
| `ocds_field_presence` | Do normalized rows include the expected OCDS identity and tender fragments? | OCID, tender identity, or linked source evidence is missing. |
| `source_locale` | Do source URL, country, language, currency, and date conventions agree? | Locale or currency conflicts with source evidence. |
| `notice_type_consistency` | Do manifest, source page, and normalized payload agree on notice semantics? | Planning, tender, award, or dynamic-market semantics conflict. |
| `expiry_plausibility` | Are publication, deadline, and expiry dates internally plausible? | Deadlines precede publication or an expired notice is claimed open. |
| `cpv_validity` | Are CPV codes well-formed and compatible with the notice family? | CPV codes are malformed or incompatible with the claimed notice. |

The overlay may flag weird-valid cases with partial optional fields, but it must not reject them unless one of the listed reject conditions is present.
