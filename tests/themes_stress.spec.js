import { test, expect } from '@playwright/test';

test.describe('Amana Level 2: Theme Preset Stress Test', () => {
  // Helper to extract computed theme custom properties directly
  async function getThemeVariables(page) {
    return await page.evaluate(() => {
      const computed = getComputedStyle(document.documentElement);
      
      return {
        colorPrimary: computed.getPropertyValue('--color-primary').trim(),
        surfaceBase: computed.getPropertyValue('--surface-base').trim(),
        surfaceElevated: computed.getPropertyValue('--surface-elevated').trim(),
        textPrimary: computed.getPropertyValue('--text-primary').trim(),
        borderSubtle: computed.getPropertyValue('--border-subtle').trim()
      };
    });
  }

  // 1. Test Luxury Theme Preset
  test('verify Luxury theme styles and colors', async ({ page }) => {
    await page.goto('http://localhost:3005/theme_luxury');
    await page.waitForSelector('.amana-card');

    const vars = await getThemeVariables(page);
    
    // Luxury theme is dark mode, primary amber (gold), canvas #050507, base #0e0e11, elevated #16161b, border gold/amber
    expect(vars.surfaceBase).toBe('#0e0e11');
    expect(vars.surfaceElevated).toBe('#16161b');
    expect(vars.textPrimary).toBe('#f4f4f5');
    // Gold/amber tint
    expect(vars.borderSubtle).toContain('rgba(234,179,8');
  });

  // 2. Test Stripe Theme Preset
  test('verify Stripe theme styles and colors', async ({ page }) => {
    await page.goto('http://localhost:3005/theme_stripe');
    await page.waitForSelector('.amana-card');

    const vars = await getThemeVariables(page);

    // Stripe theme is light mode, primary violet, canvas #f6f9fc, base/elevated #ffffff, text #0a2540, border #e6ebf1
    expect(vars.surfaceBase).toBe('#ffffff');
    expect(vars.surfaceElevated).toBe('#ffffff');
    expect(vars.textPrimary).toBe('#0a2540');
    expect(vars.borderSubtle).toBe('#e6ebf1');
  });

  // 3. Test Linear Theme Preset
  test('verify Linear theme styles and colors', async ({ page }) => {
    await page.goto('http://localhost:3005/theme_linear');
    await page.waitForSelector('.amana-card');

    const vars = await getThemeVariables(page);

    // Linear theme is dark mode, primary indigo, canvas #020204, base #08070b, elevated #121016, text #f8fafc, border white 8%
    expect(vars.surfaceBase).toBe('#08070b');
    expect(vars.surfaceElevated).toBe('#121016');
    expect(vars.textPrimary).toBe('#f8fafc');
    expect(vars.borderSubtle).toBe('rgba(255,255,255,0.08)');
  });
});
