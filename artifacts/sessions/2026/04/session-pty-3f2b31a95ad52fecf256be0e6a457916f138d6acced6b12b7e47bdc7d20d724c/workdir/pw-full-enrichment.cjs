const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const DOWNLOAD_ROOT = "artifacts/downloads/procontract";

async function dismissCookies(page) {
  // Try multiple approaches to dismiss cookie consent
  try {
    // 1) Click "Accept all" button
    const acceptBtn = page.locator('button:has-text("Accept all"), a:has-text("Accept all")').first();
    if (await acceptBtn.isVisible({ timeout: 2000 })) {
      await acceptBtn.click({ force: true });
      await page.waitForTimeout(1000);
      return;
    }
  } catch (e) {}
  
  // 2) Force-remove the dialog via JS
  await page.evaluate(() => {
    document.querySelectorAll('.cookie-dialog, .js-cookie-consent-dialog, [class*="cookie-consent"]').forEach(el => el.remove());
    document.querySelectorAll('.dialog-overlay, .modal-backdrop, [class*="overlay"]').forEach(el => el.remove());
  });
  await page.waitForTimeout(500);
}

async function login(page) {
  await page.goto("https://procontract.due-north.com/Login/Index", {
    waitUntil: "domcontentloaded",
    timeout: 30000,
  });
  await page.waitForTimeout(2000);
  await dismissCookies(page);
  
  await page.fill('input[type="text"]', "sales@sorellon.com");
  await page.fill('input[type="password"]', "9928fJG3!5nk6nc");
  await page.click('input[type="submit"], button[type="submit"]', { timeout: 5000 });
  await page.waitForTimeout(5000);
  await dismissCookies(page);
  console.log("Logged in. URL:", page.url());
}

async function collectActivitiesFromCurrentPage(page) {
  return await page.evaluate(() => {
    const activities = [];
    // Find the activities table/grid - look for the one with activity data, not cookie tables
    const tables = document.querySelectorAll('table');
    for (const table of tables) {
      if (table.className.includes('cookie')) continue;
      const headers = Array.from(table.querySelectorAll('th')).map(h => h.textContent.trim());
      // Check if this looks like the right table
      if (headers.length === 0) continue;
      
      const rows = table.querySelectorAll('tbody tr, tr');
      for (const row of rows) {
        const cells = row.querySelectorAll('td');
        if (cells.length === 0) continue;
        
        const links = [];
        row.querySelectorAll('a[href]').forEach(a => {
          links.push({ text: a.textContent.trim(), href: a.getAttribute('href') });
        });
        
        const cellTexts = Array.from(cells).map(c => c.textContent.trim());
        activities.push({ cells: cellTexts, links, headers });
      }
    }
    
    // Also look for div-based activity grid (some ProContract portals use divs)
    const gridRows = document.querySelectorAll('[class*="activity"], [class*="kgRow"], .row-item');
    gridRows.forEach(r => {
      const text = r.textContent.trim();
      const links = [];
      r.querySelectorAll('a[href]').forEach(a => {
        links.push({ text: a.textContent.trim(), href: a.getAttribute('href') });
      });
      if (links.length > 0 && text.length > 10) {
        activities.push({ cells: [text], links, source: 'div-grid' });
      }
    });
    
    return activities;
  });
}

async function hasNextPage(page) {
  return await page.evaluate(() => {
    // Look for "Next >" link or next page number
    const links = document.querySelectorAll('a');
    for (const a of links) {
      const text = a.textContent.trim();
      if (text === 'Next' || text === 'Next >' || text === '>' || text === '>>') {
        // Check it's not disabled
        if (!a.classList.contains('disabled') && !a.parentElement.classList.contains('disabled')) {
          return true;
        }
      }
    }
    return false;
  });
}

async function clickNextPage(page) {
  const nextLink = page.locator('a:has-text("Next"), a:has-text("Next >")').first();
  await nextLink.click();
  await page.waitForTimeout(2000);
}

(async () => {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1600, height: 900 },
    acceptDownloads: true,
  });
  const page = await context.newPage();

  await login(page);
  await page.screenshot({ path: "enrichment-01-home.png", scale: "css" });

  // ===== STEP 2: Navigate to "Recently added" =====
  // The home page shows Activities with tabs: Active, Recently added, Last viewed
  // Let me first check what the home page currently shows
  
  // Try clicking the "Recently added" link/tab
  try {
    // Try various selectors for the tab
    const tabSelectors = [
      'a:has-text("Recently added")',
      'button:has-text("Recently added")',
      'li:has-text("Recently added") a',
      '[data-toggle="tab"]:has-text("Recently added")',
    ];
    let clicked = false;
    for (const sel of tabSelectors) {
      try {
        const el = page.locator(sel).first();
        if (await el.isVisible({ timeout: 2000 })) {
          await el.click({ timeout: 3000 });
          clicked = true;
          console.log("Clicked Recently added via:", sel);
          break;
        }
      } catch (e) {}
    }
    if (!clicked) {
      console.log("Could not click Recently added tab. Trying text click...");
      // Force click by coordinates or evaluate
      await page.evaluate(() => {
        const els = document.querySelectorAll('a, button, span, li');
        for (const el of els) {
          if (el.textContent.trim() === 'Recently added') {
            el.click();
            return true;
          }
        }
        return false;
      });
    }
    await page.waitForTimeout(3000);
  } catch (e) {
    console.log("Tab click error:", e.message.substring(0, 100));
  }

  await page.screenshot({ path: "enrichment-02-recently-added.png", scale: "css" });

  // ===== Collect activities from ALL pages =====
  const allActivities = [];
  let currentPage = 1;
  const maxPages = 5;

  while (currentPage <= maxPages) {
    console.log(`\n--- Collecting page ${currentPage} ---`);
    const pageActivities = await collectActivitiesFromCurrentPage(page);
    console.log(`Found ${pageActivities.length} activity rows on page ${currentPage}`);
    
    for (const act of pageActivities) {
      console.log("  Activity:", JSON.stringify(act.cells.map(c => c.substring(0, 60))));
      console.log("  Links:", JSON.stringify(act.links.map(l => ({ text: l.text.substring(0, 60), href: l.href }))));
      allActivities.push({ ...act, page: currentPage });
    }

    // Check for next page
    const hasNext = await hasNextPage(page);
    console.log("Has next page:", hasNext);
    if (hasNext && currentPage < maxPages) {
      await clickNextPage(page);
      currentPage++;
      await page.screenshot({ path: `enrichment-02-page${currentPage}.png`, scale: "css" });
    } else {
      break;
    }
  }

  console.log(`\nTotal activities collected across ${currentPage} pages: ${allActivities.length}`);

  // Now we also need to look at the Active tab for existing RFx responses
  // Check if there are activities in the Active tab too
  console.log("\n--- Checking Active tab ---");
  try {
    await page.evaluate(() => {
      const els = document.querySelectorAll('a, button, span, li');
      for (const el of els) {
        if (el.textContent.trim() === 'Active') {
          el.click();
          return true;
        }
      }
      return false;
    });
    await page.waitForTimeout(2000);
    const activeActivities = await collectActivitiesFromCurrentPage(page);
    console.log(`Active tab has ${activeActivities.length} activities`);
    for (const act of activeActivities) {
      console.log("  Active:", JSON.stringify(act.cells.map(c => c.substring(0, 60))));
      console.log("  Links:", JSON.stringify(act.links.map(l => ({ text: l.text.substring(0, 60), href: l.href }))));
    }
    // Mark active activities 
    activeActivities.forEach(a => { a.source = 'active_tab'; allActivities.push(a); });
  } catch (e) {
    console.log("Active tab error:", e.message.substring(0, 100));
  }

  // Save intermediate data
  fs.writeFileSync("activities-collected.json", JSON.stringify(allActivities, null, 2));
  console.log("\nSaved activities to activities-collected.json");

  await browser.close();
  console.log("DONE_COLLECTION");
})().catch(e => {
  console.error("FATAL:", e.message);
  process.exit(1);
});
