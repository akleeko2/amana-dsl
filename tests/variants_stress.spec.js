import { test, expect } from '@playwright/test';

test.describe('Amana Level 2: Variants Stress Test', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:3005/variants_stress');
  });

  // 1. Check CSS Classes
  test('verify correct CSS classes for all variants', async ({ page }) => {
    // Button variants
    await expect(page.locator('.amana-btn.amana-btn-primary').first()).toBeVisible();
    await expect(page.locator('.amana-btn.amana-btn-secondary').first()).toBeVisible();
    await expect(page.locator('.amana-btn.amana-btn-outline').first()).toBeVisible();
    await expect(page.locator('.amana-btn.amana-btn-ghost').first()).toBeVisible();
    await expect(page.locator('.amana-btn.amana-btn-link').first()).toBeVisible();

    // Card variants
    await expect(page.locator('.amana-card-variant-flat').first()).toBeVisible();
    await expect(page.locator('.amana-card-variant-elevated').first()).toBeVisible();
    await expect(page.locator('.amana-card-variant-outlined').first()).toBeVisible();
    await expect(page.locator('.amana-card-variant-glass').first()).toBeVisible();
  });

  // 2. Check HTML Output
  test('verify correct HTML output and form nesting', async ({ page }) => {
    // Buttons are rendered as button tag
    const button = page.locator('button.amana-btn-primary').first();
    await expect(button).toBeVisible();
    expect(await button.evaluate(el => el.tagName.toLowerCase())).toBe('button');

    // Cards are rendered as article tag
    const card = page.locator('.amana-card-variant-flat').first();
    expect(await card.evaluate(el => el.tagName.toLowerCase())).toBe('article');

    // Verify AuthPage elements
    const authForm = page.locator('.amana-auth-form');
    await expect(authForm).toBeVisible();
    
    const emailInput = authForm.locator('input[type="email"]');
    await expect(emailInput).toBeVisible();
    await expect(emailInput).toHaveClass(/amana-input/);
    
    const passwordInput = authForm.locator('input[type="password"]');
    await expect(passwordInput).toBeVisible();
    await expect(passwordInput).toHaveClass(/amana-input/);
  });

  // 3. Check Responsive Layouts
  test('verify layout responsiveness across devices', async ({ page }) => {
    const grid = page.locator('.amana-grid').first();

    // Desktop: 4 columns
    await page.setViewportSize({ width: 1280, height: 800 });
    await expect(grid).toBeVisible();
    const desktopGridCols = await grid.evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(' ').length);
    expect(desktopGridCols).toBe(4);

    // Mobile: Should adapt/wrap
    await page.setViewportSize({ width: 375, height: 667 });
    const mobileGridCols = await grid.evaluate((el) => getComputedStyle(el).gridTemplateColumns.split(' ').length);
    expect(mobileGridCols).toBeLessThanOrEqual(2);
  });

  // 4. Check Dark Mode styling
  test('verify theme variables and text colors in Dark Mode', async ({ page }) => {
    await page.evaluate(() => {
      document.documentElement.classList.add('dg-mode-dark');
    });

    await page.waitForTimeout(100);

    const darkBg = await page.evaluate(() => {
      const computed = getComputedStyle(document.documentElement);
      return {
        surfaceBase: computed.getPropertyValue('--surface-base').trim(),
        textPrimary: computed.getPropertyValue('--text-primary').trim(),
      };
    });

    expect(darkBg.surfaceBase).toBe('#0b1020');
    expect(darkBg.textPrimary).toBe('#f8fafc');
  });

  // 5. Check RTL direction
  test('verify correct HTML dir attribute and RTL alignment', async ({ page }) => {
    const dirAttr = await page.evaluate(() => document.documentElement.getAttribute('dir'));
    expect(dirAttr).toBe('rtl');

    const label = page.locator('.amana-label').first();
    const textAlign = await label.evaluate((el) => getComputedStyle(el).textAlign);
    expect(textAlign).toBe('start');
  });
});
