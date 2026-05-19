#!/usr/bin/env bash

set -euo pipefail

pass() { printf '[PASS] %s\n' "$1"; }
warn() { printf '[WARN] %s\n' "$1"; }
fail() { printf '[FAIL] %s\n' "$1"; exit 1; }

repo_root="${L5_PREFLIGHT_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

require_file() {
  local path="$1"
  test -f "$path" || fail "required file missing: $path"
  pass "file present: $path"
}

require_command() {
  local command_name="$1"
  command -v "$command_name" >/dev/null 2>&1 || fail "$command_name missing"
  pass "command available: $command_name"
}

warn_command() {
  local command_name="$1"
  if command -v "$command_name" >/dev/null 2>&1; then
    pass "command available: $command_name"
  else
    warn "$command_name unavailable; install before commit gate"
  fi
}

forbidden_paid_key_vars=(
  XE_API_KEY
  FREIGHTOS_API_KEY
  SEARATES_API_KEY
  TARIC_SUPPORT_API_KEY
  HS_PING_API_KEY
  EASYSHIP_API_KEY
  SHIPPO_API_KEY
  CURRENCYBEACON_API_KEY
  OANDA_API_KEY
  DUTIFY_API_KEY
  ZONOS_API_KEY
  AVALARA_API_KEY
)

for name in "${forbidden_paid_key_vars[@]}"; do
  if test -n "${!name:-}"; then
    fail "forbidden paid API env var is set: $name"
  fi
done
pass 'no paid commercial API key env vars detected'

require_command cargo
require_command bun
require_command rg
require_command curl
require_command jq
require_command rustup
warn_command ubs

rustup toolchain list | rg -q 'nightly' || fail 'Rust nightly toolchain missing'
pass 'Rust nightly toolchain detected'

require_file docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
require_file docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
require_file docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md
require_file docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md
require_file infra/supabase/migrations/0001_projection_tables.sql
require_file infra/supabase/migrations/0004_rls_policies.sql
require_file crates/daemon/src/db/sync.rs
require_file bin/ensure-cdp-9222

test -x bin/ensure-cdp-9222 || fail 'bin/ensure-cdp-9222 is not executable'
pass 'bin/ensure-cdp-9222 is executable'

rg -q 'landed_cost_estimates' crates/daemon/src/db/sync.rs \
  || fail 'landed_cost_estimates missing from sync push set'
pass 'landed_cost_estimates is present in sync push set'

if rg -q 'product_commodity_codes' crates/daemon/src/db/sync.rs; then
  pass 'product_commodity_codes is present in sync push set'
else
  warn 'product_commodity_codes not yet in sync push set; l5-p1-sqlite-product-commodity-codes owns this Phase 1 lane'
fi

rg -q 'product_commodity_codes' infra/supabase/migrations/0001_projection_tables.sql \
  || fail 'product_commodity_codes missing from Supabase projection migration'
pass 'product_commodity_codes Supabase projection exists'

rg -q 'product_commodity_codes' infra/supabase/migrations/0004_rls_policies.sql \
  || fail 'product_commodity_codes missing from RLS policy migration'
pass 'product_commodity_codes RLS policy exists'

hmrc_base="${HMRC_TRADE_TARIFF_BASE_URL:-https://www.trade-tariff.service.gov.uk}"
commodity_probe="${hmrc_base}/api/v2/commodities/7008002000?filter[geographical_area_id]=CN"
legacy_probe="${hmrc_base}/uk/api/commodities/7008002000?filter[geographical_area_id]=CN"
search_probe="${hmrc_base}/api/v2/search?q=screws"
commodity_cassette="tests/fixtures/cassettes/l5/hmrc/commodity-7008002000-cn.json"
hmrc_fx_cassette="tests/fixtures/cassettes/l5/fx/hmrc-monthly-current.json"
frankfurter_cassette="tests/fixtures/cassettes/l5/fx/frankfurter-usd-gbp.json"

if curl -g -fsS "$commodity_probe" | jq -e '.data.attributes.goods_nomenclature_item_id' >/dev/null; then
  pass 'HMRC Trade Tariff v2 commodity probe ok'
elif curl -g -fsS "$legacy_probe" | jq -e '.data.attributes.goods_nomenclature_item_id' >/dev/null; then
  pass 'HMRC Trade Tariff legacy commodity probe ok'
elif test -f "$commodity_cassette"; then
  warn 'HMRC commodity probe failed; cassette fallback present for deterministic tests'
else
  fail 'HMRC Trade Tariff commodity probe failed'
fi

curl -g -fsS "$search_probe" >/dev/null \
  && pass 'HMRC search probe ok' \
  || warn 'HMRC search probe failed; endpoint discovery bead must resolve search path'

current_year="$(date -u +%Y)"
hmrc_fx_url="${HMRC_EXCHANGE_RATES_URL:-https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=${current_year}}"
if curl -fsS "$hmrc_fx_url" | rg -q 'monthly exchange rates|Monthly exchange rates'; then
  pass 'HMRC monthly exchange-rate publication reachable'
elif test -f "$hmrc_fx_cassette"; then
  warn 'HMRC monthly exchange-rate publication unreachable; cassette fallback present'
else
  fail 'HMRC monthly exchange-rate publication unreachable'
fi

frankfurter_base="${FRANKFURTER_BASE_URL:-https://api.frankfurter.app}"
if curl -LfsS "${frankfurter_base}/latest?from=USD&to=GBP" | jq -e '.rates.GBP' >/dev/null; then
  pass 'Frankfurter public latest-rate API reachable'
elif test -f "$frankfurter_cassette"; then
  warn 'Frankfurter latest-rate probe failed; cassette fallback present'
else
  fail 'Frankfurter latest-rate probe failed'
fi

cdp_port="${AGENT_BROWSER_CDP_PORT:-${CDP_PORT:-9222}}"
helper="${CDP_ENSURE_HELPER_PATH:-${repo_root}/bin/ensure-cdp-9222}"
test -x "$helper" || fail "CDP helper missing or not executable: $helper"
"$helper" >/dev/null 2>&1 || fail 'CDP helper failed to launch or verify Chrome'
curl -fsS "http://127.0.0.1:${cdp_port}/json/version" | jq -e '.Browser' >/dev/null \
  || fail "CDP browser not reachable on port ${cdp_port}"
pass "CDP browser reachable on port ${cdp_port}"

for file in \
  skills/landed-cost/hmrc-customs-classification/SKILL.md \
  skills/landed-cost/simple-fx-lookup/SKILL.md \
  skills/landed-cost/searates-freight-estimate/SKILL.md \
  skills/landed-cost/freightos-freight-estimate/SKILL.md \
  skills/landed-cost/freightos-freight-estimate/result.schema.json \
  skills/landed-cost/taric-cross-check/SKILL.md \
  skills/landed-cost/taric-cross-check/result.schema.json \
  skills/supplier-price-enrichment/alibaba/SKILL.md \
  skills/supplier-price-enrichment/1688/SKILL.md \
  skills/supplier-price-enrichment/amazon/SKILL.md \
  skills/supplier-price-enrichment/ebay/SKILL.md
do
  require_file "$file"
done

for report in \
  docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/TEST_REPORT.md \
  docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/taric-cross-check/TEST_REPORT.md
do
  require_file "$report"
  rg -q 'STATUS: READY' "$report" || fail "operator-built prerequisite report is not READY: $report"
  pass "operator-built prerequisite report READY: $report"
done

for dir in \
  skills/supplier-price-enrichment/alibaba \
  skills/supplier-price-enrichment/1688 \
  skills/supplier-price-enrichment/amazon \
  skills/supplier-price-enrichment/ebay \
  skills/procurement-ingestion/pcs
do
  for file in SKILL.md result.schema.json checkpoint.schema.json canary.inputs.jsonl golden.assertions.yaml validator-rubric.yaml; do
    if test -f "$dir/$file"; then
      pass "production skill artifact present: $dir/$file"
    else
      warn "production skill artifact missing until L5 promotion: $dir/$file"
    fi
  done
done

for category in fasteners packaging-cartons examination-gloves; do
  base="tests/fixtures/calibration/customs-classification/${category}"
  if test -d "$base"; then
    pass "customs calibration category dir present: $base"
  else
    warn "customs calibration category dir missing until Phase 6: $base"
    continue
  fi
  for file in canary.inputs.jsonl golden.assertions.yaml metadata.json manifest.md; do
    if test -f "$base/$file"; then
      pass "customs calibration artifact present: $base/$file"
    else
      warn "customs calibration artifact missing until Phase 6: $base/$file"
    fi
  done
done

test -w "${TMPDIR:-/tmp}" || fail 'temp filesystem not writable'
pass 'temp filesystem writable'
pass 'L5 preflight completed'
