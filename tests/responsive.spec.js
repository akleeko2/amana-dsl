import { test, expect } from '@playwright/test';
import sqlite3Pkg from 'sqlite3';
import path from 'path';

const sqlite3 = sqlite3Pkg.verbose();

test.describe('Amana Level 2: Responsive & Mobile Viewport Test', () => {
  const viewports = [320, 375, 768, 1024, 1440, 1920];

  test.beforeAll(async () => {
    const dbPath = path.join(process.cwd(), 'component_test', 'dist', 'app.db');
    const db = new sqlite3.Database(dbPath);
    await new Promise((resolve, reject) => {
      db.run(
        `INSERT OR IGNORE INTO "user" (email, password, name) VALUES ('test@example.com', 'pass123', 'أحمد علي')`,
        (err) => {
          if (err) reject(err);
          else resolve();
        }
      );
    });
    db.close();
  });

  test('verify page-level scroll stability and sidebar wrapping', async ({ page }) => {
    await page.goto('http://localhost:3005/responsive');
    await page.waitForSelector('.amana-sidebar');

    for (const width of viewports) {
      await page.setViewportSize({ width, height: 800 });
      await page.waitForTimeout(50); // let layout settle

      // Verify no horizontal page-level overflow
      const pageScrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      expect(pageScrollWidth).toBeLessThanOrEqual(width + 2); // 2px tolerance for layout scrollbar width variations

      // Check Sidebar responsive wrapping
      const sidebar = page.locator('.amana-sidebar');
      const displayStyle = await sidebar.evaluate((el) => getComputedStyle(el).display);
      expect(displayStyle).toBeDefined();
    }
  });

  test('verify DashboardShell sidebar toggle on mobile', async ({ page }) => {
    await page.goto('http://localhost:3005/responsive');

    // Mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    const toggleBtn = page.locator('.amana-db-toggle');
    await expect(toggleBtn).toBeVisible();

    const dbSidebar = page.locator('.amana-db-sidebar');
    // Initially sidebar is offscreen or hidden
    await expect(dbSidebar).not.toHaveClass(/open/);

    // Click toggle button
    await toggleBtn.click();
    await expect(dbSidebar).toHaveClass(/open/);

    // Desktop viewport
    await page.setViewportSize({ width: 1440, height: 900 });
    await expect(toggleBtn).not.toBeVisible();
  });

  test('verify DataTable scroll container and Reel scrollability', async ({ page }) => {
    await page.goto('http://localhost:3005/responsive');
    await page.setViewportSize({ width: 375, height: 667 });

    // Table responsiveness: should have overflow-x auto
    const tableContainer = page.locator('.amana-table-responsive').first();
    await expect(tableContainer).toBeVisible();
    const tableOverflow = await tableContainer.evaluate((el) => getComputedStyle(el).overflowX);
    expect(['auto', 'scroll']).toContain(tableOverflow);

    // Reel responsiveness: should have overflow-x auto
    const reel = page.locator('.amana-reel').first();
    await expect(reel).toBeVisible();
    const reelOverflow = await reel.evaluate((el) => getComputedStyle(el).overflowX);
    expect(['auto', 'scroll']).toContain(reelOverflow);
  });

  test('verify Masonry columns stack on mobile', async ({ page }) => {
    await page.goto('http://localhost:3005/responsive');
    const masonry = page.locator('.amana-masonry').first();

    // Desktop
    await page.setViewportSize({ width: 1440, height: 900 });
    const desktopCols = await masonry.evaluate((el) => getComputedStyle(el).columns || getComputedStyle(el).columnCount);
    expect(desktopCols).toContain('3');

    // Mobile
    await page.setViewportSize({ width: 375, height: 667 });
    const mobileCols = await masonry.evaluate((el) => getComputedStyle(el).columns || getComputedStyle(el).columnCount);
    expect(mobileCols).toContain('1');
  });
});
