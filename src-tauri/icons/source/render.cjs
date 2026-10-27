const { chromium } = require('playwright');
(async () => {
  const fs = require('fs');
  const svg = fs.readFileSync(process.argv[2], 'utf8');
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
  await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
  await page.screenshot({ path: process.argv[3], omitBackground: true, clip: { x: 0, y: 0, width: 1024, height: 1024 } });
  await browser.close();
})();
