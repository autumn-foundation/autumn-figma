// Browser smoke test for the todo app made from the Figma fixture.
//
// Start the app first, then run: BASE_URL=http://127.0.0.1:3000 npm run smoke
// The test adds a todo with the Figma "Add" button, deletes it with a
// double-click, and checks the out-of-band count. It fails on any console
// error, for example a CSP violation.
import { chromium } from 'playwright';

const base = process.env.BASE_URL ?? 'http://127.0.0.1:3000';
const fail = (message) => { console.error(`FAIL: ${message}`); process.exit(1); };

// CHROMIUM_PATH picks a local browser; CI installs the matching one.
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
const page = await browser.newPage();
const problems = [];
page.on('console', (m) => { if (m.type() === 'error') problems.push(m.text()); });
page.on('pageerror', (e) => problems.push(e.message));

const response = await page.goto(`${base}/`);
if (response.status() !== 200) fail(`GET / gave ${response.status()}`);
const rows = () => page.locator('#todo-list > li').count();
const before = await rows();
const countBefore = await page.locator('#count').innerText();

await page.fill('input[name=title]', 'Smoke test');
await page.click('form button');
await page.waitForFunction((n) => document.querySelectorAll('#todo-list > li').length === n + 1, before);
if ((await page.locator('#todo-list > li').last().innerText()).trim() !== 'Smoke test') fail('the new row has the wrong text');
if ((await page.locator('#count').innerText()) === countBefore) fail('the count did not change');

await page.locator('#todo-list > li').last().dblclick();
await page.waitForFunction((n) => document.querySelectorAll('#todo-list > li').length === n, before);
if ((await page.locator('#count').innerText()) !== countBefore) fail('the count did not change back');

if (problems.length > 0) fail(`console errors: ${problems.join(' | ')}`);
await browser.close();
console.log(`ok: ${before} -> ${before + 1} -> ${before} rows, count "${countBefore}"`);
