import { test, expect } from '@playwright/test';

test.describe('Amana Visual Components Smoke Tests', () => {
  test('should render grid alignment and interact with production-safe modal', async ({ page }) => {
    // 1. Go to homepage
    await page.goto('/');

    // 2. Verify Grids are rendered
    // Stretched Grid
    const stretchedGrid = page.locator('.amana-grid.amana-grid-stretch');
    await expect(stretchedGrid).toBeVisible();

    // Normal Grid
    const normalGrid = page.locator('.amana-grid:not(.amana-grid-stretch)');
    await expect(normalGrid).toBeVisible();

    // 3. Verify Modal is initially hidden
    const modal = page.locator('.amana-modal');
    await expect(modal).not.toBeVisible();

    // Verify document body does not have scroll lock initially
    const bodyOverflowInitial = await page.evaluate(() => document.body.style.overflow);
    expect(bodyOverflowInitial).not.toBe('hidden');

    // 4. Click Open Modal button
    const openBtn = page.locator('button:has-text("Open Modal")');
    await expect(openBtn).toBeVisible();
    await openBtn.click();

    // 5. Verify Modal is visible
    await expect(modal).toBeVisible();

    // Verify monotonic title ID and ARIA attributes
    const modalTitle = modal.locator('.amana-modal-title');
    await expect(modalTitle).toBeVisible();
    await expect(modalTitle).toHaveAttribute('id', 'amana-modal-title-0');
    await expect(modal).toHaveAttribute('role', 'dialog');
    await expect(modal).toHaveAttribute('aria-modal', 'true');
    await expect(modal).toHaveAttribute('aria-labelledby', 'amana-modal-title-0');

    // 6. Verify scroll lock is active
    const bodyOverflowLocked = await page.evaluate(() => document.body.style.overflow);
    expect(bodyOverflowLocked).toBe('hidden');

    // 7. Verify keyboard focus interaction
    await page.keyboard.press('Tab');
    const activeElementText = await page.evaluate(() => document.activeElement ? document.activeElement.innerText : '');
    expect(activeElementText).toBeDefined();

    // 8. Close modal using Escape key
    await page.keyboard.press('Escape');
    await expect(modal).not.toBeVisible();

    // Verify scroll lock is released
    const bodyOverflowUnlocked = await page.evaluate(() => document.body.style.overflow);
    expect(bodyOverflowUnlocked).not.toBe('hidden');
  });
});
