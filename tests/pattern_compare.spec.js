import { test, expect } from '@playwright/test';

test.describe('Amana Level 2: Pattern Expansion Test (AuthPage)', () => {
  test('verify pattern_auth matches manual_auth structural nesting', async ({ page }) => {
    // 1. Visit /pattern_auth and extract structural metadata
    await page.goto('http://localhost:3005/pattern_auth');
    await page.waitForSelector('.amana-auth-page');

    const getAuthMetadata = async (page) => {
      return page.evaluate(() => {
        const pageNode = document.querySelector('.amana-auth-page');
        const card = pageNode.querySelector('.amana-auth-card');
        const header = card.querySelector('.amana-auth-header');
        const title = header.querySelector('h2').textContent.trim();
        const body = card.querySelector('.amana-auth-body');
        const form = body.querySelector('.amana-auth-form');
        
        const action = form.getAttribute('action');
        const method = form.getAttribute('method').toUpperCase();

        const fields = Array.from(form.querySelectorAll('.amana-field')).map(field => {
          const label = field.querySelector('.amana-label').textContent.trim();
          const input = field.querySelector('.amana-input');
          return {
            label,
            type: input.getAttribute('type'),
            name: input.getAttribute('name'),
            required: input.hasAttribute('required')
          };
        });

        const button = form.querySelector('.amana-btn');
        const btnText = button.querySelector('span') ? button.querySelector('span').textContent.trim() : button.textContent.trim();
        const btnClasses = Array.from(button.classList);

        return {
          title,
          action,
          method,
          fields,
          btnText,
          btnClasses: btnClasses.filter(c => c !== 'amana-btn-intent-default') // normalize layout classes
        };
      });
    };

    const patternMetadata = await getAuthMetadata(page);

    // 2. Visit /manual_auth and extract structural metadata
    await page.goto('http://localhost:3005/manual_auth');
    await page.waitForSelector('.amana-auth-page');
    const manualMetadata = await getAuthMetadata(page);

    // 3. Assert deep equality of the structure
    expect(patternMetadata).toEqual(manualMetadata);
  });
});
