(async () => {
  const J = JOB_PARAMETERS_JSON;

  // Cities not directly served by Freightos seaports: route to nearest major port.
  // Discovered empirically 2026-05-17 when /servlet/geoPlaces returned no
  // seaport for Portsmouth (a small UK port not in Freightos' carrier network).
  const CITY_FALLBACK = {
    "GB:Portsmouth": "Southampton",
    "GB:Tilbury":    "Felixstowe",
    "GB:Greenwich":  "Felixstowe"
  };

  const PACK = { "20": "container20", "40": "container40", "40HC": "container40HC", "45HC": "container45HC" };

  const warnings = [];
  const blockers = [];

  async function resolvePort(country, city) {
    // Apply nearest-port fallback first
    const fallbackKey = `${country}:${(city || "").trim()}`;
    const lookupCity = CITY_FALLBACK[fallbackKey] || city;
    if (CITY_FALLBACK[fallbackKey]) {
      warnings.push({
        code: "FREIGHTOS_PORT_FALLBACK_NEAREST",
        message: `${city}, ${country} has no Freightos seaport; routed via nearest port ${lookupCity}.`,
        severity: "warning"
      });
    }

    const url = `/servlet/geoPlaces?country=${encodeURIComponent(country)}&types=seaports&isQuotingContext=true&searchTerm=${encodeURIComponent(lookupCity)}`;
    let r;
    try {
      r = await fetch(url, { credentials: "include", signal: AbortSignal.timeout(8000) });
    } catch (e) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `geoPlaces fetch failed for ${country}/${lookupCity}: ${String(e)}`, severity: "blocked" });
      return null;
    }
    if (r.status === 401) return { __session_expired: true };
    if (!r.ok) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `geoPlaces HTTP ${r.status} for ${country}/${lookupCity}`, severity: "blocked" });
      return null;
    }
    const j = await r.json();
    const seaports = j.seaports || [];
    if (!seaports.length) {
      blockers.push({
        code: "FREIGHTOS_PORT_NOT_FOUND",
        message: `No Freightos seaport found for ${country}/${lookupCity}. Add a CITY_FALLBACK entry if a nearby port exists.`,
        severity: "blocked"
      });
      return null;
    }
    // Prefer exact englishName match if available, else first result (Freightos ranks by relevance)
    const exact = seaports.find(p => p.englishName && p.englishName.toLowerCase() === lookupCity.toLowerCase());
    const chosen = exact || seaports[0];
    return {
      locationName: "port_airport",
      locationTypeCode: "Seaport",
      locationCode: chosen.code,
      countryID: { value: country },
      _resolvedName: chosen.englishName,
      _resolvedCode: chosen.code
    };
  }

  async function runSearch({ withServices }) {
    const origin = await resolvePort(J.origin_country, J.origin_address);
    if (origin && origin.__session_expired) return { __session_expired: true };
    const dest = await resolvePort(J.destination_country, J.destination_address);
    if (dest && dest.__session_expired) return { __session_expired: true };
    if (!origin || !dest) return null;
    const goodsValue = Math.max(1, Math.round((J.goods_value_minor || 0) / 100));
    const containerType = (J.load && J.load.mode === "container") ? (PACK[J.load.container_type] || "container40") : "container40";
    // Strip internal fields from location objects sent to API
    const cleanOrigin = { locationName: origin.locationName, locationTypeCode: origin.locationTypeCode, locationCode: origin.locationCode, countryID: origin.countryID };
    const cleanDest   = { locationName: dest.locationName,   locationTypeCode: dest.locationTypeCode,   locationCode: dest.locationCode,   countryID: dest.countryID };
    const payload = {
      messageHeader: { messageID: crypto.randomUUID() },
      businessInfo: {
        serviceName: "Quoting", serviceMethod: "New",
        messageDateTime: new Date().toISOString(),
        parties: [
          { partyTypeCode: "BY", contact: { electronicMail: J.buyer_email || "dieter.x.s.danicek@gmail.com" }, knownShipper: false },
          { partyTypeCode: "FR", name: "freightos.com" }
        ]
      },
      shipment: {
        originLocation: cleanOrigin, destinationLocation: cleanDest,
        additionalInformation: [],
        pickupEvent: { eventDate: { scheduledDateTime: (J.ship_date ? new Date(J.ship_date).toISOString() : new Date(Date.now() + 86400000).toISOString()), endDateTime: "" } },
        load: {
          packages: [{ quantity: (J.load && J.load.units) || 1, packagingType: containerType, overWeightIndicator: !!(J.load && J.load.overweight) }],
          declaredValue: { value: goodsValue, currencyID: J.goods_currency || "USD" }
        },
        accessorials: [],
        // Freightos rejects mismatched insurance vs declared values
        // (HTTP 400 "Conflict between declared value and insurance value!"),
        // so always mirror goods value. The with-services delta is captured by
        // declaredCustoms.entry (customs brokerage toggle).
        insuranceValueAmount: { value: goodsValue, currencyID: J.goods_currency || "USD" },
        pricePreference: { includeOriginPortCharges: true, includeDestinationPortCharges: true },
        declaredCustoms: { entry: !!withServices }
      }
    };

    let postRes;
    try {
      postRes = await fetch("/api/open-freight/quoting/quotes/search", {
        method: "POST",
        headers: { "Content-Type": "application/json", "Accept": "application/json" },
        body: JSON.stringify(payload),
        credentials: "include",
        signal: AbortSignal.timeout(12000)
      });
    } catch (e) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `POST /search failed: ${String(e)}`, severity: "blocked" });
      return null;
    }
    if (postRes.status === 401) return { __session_expired: true };
    if (!postRes.ok) {
      const errText = await postRes.text().catch(() => "");
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `POST /search HTTP ${postRes.status}: ${errText.slice(0, 200)}`, severity: "blocked" });
      return null;
    }
    const postJson = await postRes.json();
    const rfq = postJson?.messageHeader?.conversationID;
    if (!rfq) {
      blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: "POST /search response missing conversationID", severity: "blocked" });
      return null;
    }
    const resultsUri = postJson?.URIReferences?.results?.uri || null;

    // Poll with patience — first quotes can take 5-15 seconds to arrive
    const quotes = [];
    let cursor = 0, hasMore = true, polls = 0, emptyPolls = 0;
    const deadline = Date.now() + 45000;
    while (hasMore && polls < 12 && Date.now() < deadline) {
      let r;
      try {
        r = await fetch(`/api/open-freight/quoting/quotes/${rfq}/poll?cursor=${cursor}&excludePlatformFee=true`, { credentials: "include", signal: AbortSignal.timeout(10000) });
      } catch (e) {
        blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `poll cursor=${cursor} failed: ${String(e)}`, severity: "blocked" });
        return null;
      }
      if (r.status === 401) return { __session_expired: true };
      if (!r.ok) {
        blockers.push({ code: "FREIGHTOS_API_TIMEOUT_OR_ERROR", message: `poll cursor=${cursor} HTTP ${r.status}`, severity: "blocked" });
        return null;
      }
      const j = await r.json();
      const newQuotes = Array.isArray(j.quotes) ? j.quotes : [];
      quotes.push(...newQuotes);
      cursor = (j.paging?.nextCursor !== undefined) ? j.paging.nextCursor : cursor + newQuotes.length;
      hasMore = !!j.paging?.hasMore;
      polls++;
      if (newQuotes.length === 0) {
        emptyPolls++;
        // If still hasMore but empty, wait longer for quotes to arrive
        if (hasMore && emptyPolls < 4) {
          await new Promise(r => setTimeout(r, 2500));
          continue;
        }
        if (emptyPolls >= 4) break;
      } else {
        emptyPolls = 0;
        await new Promise(r => setTimeout(r, 600));
      }
    }
    return { rfq, quotes, resultsUri, polls, origin_resolved: origin._resolvedName, origin_code: origin._resolvedCode, dest_resolved: dest._resolvedName, dest_code: dest._resolvedCode };
  }

  const baseRun = await runSearch({ withServices: false });
  if (!baseRun) return JSON.stringify({ __error: true, warnings, blockers });
  if (baseRun.__session_expired) return JSON.stringify({ __session_expired: true });

  const now = Date.now();
  const usable = baseRun.quotes.filter(q => q?.priceIndicator?.totalCharge && (new Date(q.validTo).getTime() > now));
  if (!usable.length) {
    return JSON.stringify({
      __ok: true, rfq_id: baseRun.rfq, results_uri: baseRun.resultsUri,
      total_quotes: baseRun.quotes.length,
      origin_resolved: baseRun.origin_resolved, origin_code: baseRun.origin_code,
      dest_resolved: baseRun.dest_resolved, dest_code: baseRun.dest_code,
      no_usable: true, warnings, blockers
    });
  }
  // Best Value = first non-expired (API-sorted)
  const best = usable[0];
  const cheapest = usable.slice().sort((a, b) => (a.priceIndicator.totalChargeDiscounted?.value || a.priceIndicator.totalCharge.value) - (b.priceIndicator.totalChargeDiscounted?.value || b.priceIndicator.totalCharge.value))[0];
  const priciest = usable.slice().sort((a, b) => (b.priceIndicator.totalChargeDiscounted?.value || b.priceIndicator.totalCharge.value) - (a.priceIndicator.totalChargeDiscounted?.value || a.priceIndicator.totalCharge.value))[0];

  function priceOf(q) {
    const p = q.priceIndicator.totalChargeDiscounted || q.priceIndicator.totalCharge;
    return { value: p.value, currency: p.currencyID };
  }

  let withServicesPrice = null;
  if (J.include_insurance || J.include_customs_brokerage) {
    const wsRun = await runSearch({ withServices: true });
    if (wsRun && !wsRun.__session_expired && !wsRun.__error) {
      const wsUsable = wsRun.quotes.filter(q => q?.priceIndicator?.totalCharge && (new Date(q.validTo).getTime() > Date.now()));
      if (wsUsable.length) withServicesPrice = priceOf(wsUsable[0]);
    }
  }

  return JSON.stringify({
    __ok: true,
    rfq_id: baseRun.rfq,
    results_uri: baseRun.resultsUri,
    total_quotes: baseRun.quotes.length,
    usable_count: usable.length,
    origin_resolved: baseRun.origin_resolved, origin_code: baseRun.origin_code,
    dest_resolved: baseRun.dest_resolved, dest_code: baseRun.dest_code,
    best, cheapest, priciest,
    base_price: priceOf(best),
    cheapest_price: priceOf(cheapest),
    priciest_price: priceOf(priciest),
    with_services_price: withServicesPrice,
    warnings, blockers
  });
})()
