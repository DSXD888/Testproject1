# UK Installation Labour Canary Corpus

This fixture mirrors the committed `landed-cost/uk-installation-labour-estimate` worker canary corpus for stable calibration and promotion checks.

## Scenarios

- Scenario CAN-LAB-001: `lights-nw-2day-01` pins the North West festive-lighting two-day install/de-install case, the inclusive `210000..=265000` labour band, electrician/labourer trades, operated boom MEWP, unknown permit responsibility, and `PERMIT_RESPONSIBILITY_UNKNOWN`.
- Scenario CAN-LAB-002: `lights-lon-2day-01` pins the London Regent Street variant, the `14000/9300` London-to-NW multiplier relationship, and both `LANE_RENTAL_ZONE` and `TTRO_LEAD_TIME_RISK`.
- Scenario CAN-LAB-003: `cctv-yh-install-01` pins the Yorkshire CCTV/security install route, the `8800` bps regional multiplier, security/electrical installer trades, and at-height access-equipment selection.
- Scenario CAN-LAB-004: `furniture-lon-assembly-01` pins the London flat-pack furniture assembly route, handyperson/fitter labour, and the no-access-equipment assertion.
- Scenario CAN-LAB-005: `region-unknown-01` pins unknown-region fallback to national `10000` bps with `LABOUR_REGION_UNKNOWN` while still producing an estimated result.
- Scenario CAN-LAB-006: `scope-unresolved-01` pins fail-soft unresolved scope, lowered confidence, review routing, and the missing maintenance term default with `MAINTENANCE_TERM_ASSUMED`.
- Scenario CAN-LAB-007: `rate-stale-01` pins stale rate-table handling with `RATE_TABLE_STALE` and a retained three-point estimate.

## Warning Coverage

Scenario coverage for CAN-WARN-001 pins all eight warning codes: `LABOUR_REGION_UNKNOWN`, `LABOUR_SCOPE_UNRESOLVED`, `ACCESS_AT_HEIGHT_ASSUMED_MEWP`, `MAINTENANCE_TERM_ASSUMED`, `RATE_TABLE_STALE`, `PERMIT_RESPONSIBILITY_UNKNOWN`, `LANE_RENTAL_ZONE`, and `TTRO_LEAD_TIME_RISK`.
