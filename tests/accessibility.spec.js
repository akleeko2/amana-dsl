import { test, expect } from '@playwright/test';

test.describe('Amana Level 2: Accessibility & Keyboard Navigation Test', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:3005/accessibility');
  });

  test('verify tab focus navigation and accordion keyboard toggle', async ({ page }) => {
    // Tab to the first button (Open Modal)
    await page.keyboard.press('Tab');
    const openModalBtn = page.locator('button:has-text("فتح المودال")');
    await expect(openModalBtn).toBeFocused();

    // Tab to the second button (Open CommandPalette)
    await page.keyboard.press('Tab');
    const openCPBtn = page.locator('button:has-text("فتح باليت البحث")');
    await expect(openCPBtn).toBeFocused();

    // Tab to Accordion Header 1
    await page.keyboard.press('Tab');
    const accHeader1 = page.locator('#amana-acc-btn-0-0');
    await expect(accHeader1).toBeFocused();
    await expect(page.locator('#amana-acc-panel-0-0')).not.toBeVisible();

    // Expand Accordion Header 1 via Enter
    await page.keyboard.press('Enter');
    await expect(page.locator('#amana-acc-panel-0-0')).toBeVisible();

    // Collapse Accordion Header 1 via Space
    await page.keyboard.press('Space');
    await expect(page.locator('#amana-acc-panel-0-0')).not.toBeVisible();
  });

  test('verify tab component arrow key navigation', async ({ page }) => {
    // Focus first tab button
    const tab1 = page.locator('#amana-tab-btn-0-0');
    await tab1.focus();
    await expect(tab1).toBeFocused();
    await expect(page.locator('#amana-tab-panel-0-0')).toBeVisible();

    // Press ArrowRight or ArrowLeft to switch to Tab 2
    await page.keyboard.press('ArrowRight');
    const tab2 = page.locator('#amana-tab-btn-0-1');
    await expect(tab2).toBeFocused();
    await expect(page.locator('#amana-tab-panel-0-1')).toBeVisible();
  });

  test('verify modal keyboard toggle and escape close', async ({ page }) => {
    // Click Open Modal to trigger it
    const openBtn = page.locator('button:has-text("فتح المودال")');
    await openBtn.click();

    const modal = page.locator('.amana-modal');
    await expect(modal).toBeVisible();

    // Press Escape to close it
    await page.keyboard.press('Escape');
    await expect(modal).not.toBeVisible();
  });

  test('verify dropdown toggle and escape close', async ({ page }) => {
    const trigger = page.locator('.amana-dropdown-trigger');
    await trigger.focus();
    await expect(trigger).toBeFocused();

    // Press Enter to open
    await page.keyboard.press('Enter');
    const menu = page.locator('.amana-dropdown-menu');
    await expect(menu).toBeVisible();

    // Press Escape to close
    await page.keyboard.press('Escape');
    await expect(menu).not.toBeVisible();
  });

  test('verify command palette escape close', async ({ page }) => {
    const openBtn = page.locator('button:has-text("فتح باليت البحث")');
    await openBtn.click();

    const cp = page.locator('.amana-command-palette-backdrop');
    await expect(cp).toBeVisible();

    // Input should be focused automatically
    const input = cp.locator('.amana-cp-input');
    await expect(input).toBeFocused();

    // Press Escape to close
    await page.keyboard.press('Escape');
    await expect(cp).not.toBeVisible();
  });
});
