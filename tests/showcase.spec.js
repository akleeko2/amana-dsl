import { test, expect } from '@playwright/test';

test.describe('Amana Showcase Visual & Console Tests', () => {
  test('verify showcase page elements and ensure no console errors', async ({ page }) => {
    const consoleErrors = [];
    const failedRequests = [];

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    page.on('requestfailed', (request) => {
      failedRequests.push(`${request.url()}: ${request.failure()?.errorText || 'Failed'}`);
    });

    page.on('response', (response) => {
      if (response.status() >= 400) {
        failedRequests.push(`${response.url()}: Status ${response.status()}`);
      }
    });

    console.log('Visiting http://localhost:3005/showcase...');
    const response = await page.goto('http://localhost:3005/showcase');

    // 1. Verify HTML generation
    expect(response.status()).toBe(200);
    const htmlContent = await page.content();
    expect(htmlContent).toContain('<html');
    
    // 2. Verify visual presence of key components (using .first() for elements with multiple occurrences)
    await expect(page.locator('.amana-container').first()).toBeVisible();
    await expect(page.locator('.amana-navbar').first()).toBeVisible();
    await expect(page.locator('.amana-banner').first()).toBeVisible();
    await expect(page.locator('.amana-hero').first()).toBeVisible();
    await expect(page.locator('.amana-btn-primary').first()).toBeVisible();
    await expect(page.locator('.amana-sidebar').first()).toBeVisible();
    await expect(page.locator('.amana-filterbar').first()).toBeVisible();
    await expect(page.locator('.amana-grid').first()).toBeVisible();
    
    // Check custom components
    await expect(page.locator('.amana-feature-card').first()).toBeVisible();
    await expect(page.locator('.amana-pricing-card').first()).toBeVisible();
    await expect(page.locator('.amana-slides').first()).toBeVisible();
    
    // Check that there are no console errors during visual rendering
    console.log('Captured JS console errors:', consoleErrors);
    expect(consoleErrors).toEqual([]);

    // Check that there are no failed network requests (e.g. 404 image errors)
    console.log('Captured failed network requests:', failedRequests);
    expect(failedRequests).toEqual([]);
    
    // 3. Verify current dir attribute
    const dirAttr = await page.evaluate(() => document.documentElement.dir);
    console.log('Current HTML dir:', dirAttr);
  });
});
