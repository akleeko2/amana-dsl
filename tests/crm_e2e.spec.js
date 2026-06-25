import { test, expect } from '@playwright/test';

test.describe('Amana CRM End-to-End User Journey Tests', () => {
  test('verify full CRUD, relationships, and reports', async ({ page }) => {
    // Enable console logs capture to aid debugging if anything fails
    page.on('console', msg => {
      console.log(`[Browser Console] ${msg.type()}: ${msg.text()}`);
    });

    // 1. Visit Login Page
    console.log('1. Navigating to http://localhost:3009/login ...');
    await page.goto('http://localhost:3009/login');
    await expect(page.locator('form')).toBeVisible();

    // 2. Fill credentials & Login
    console.log('2. Entering admin credentials ...');
    await page.fill('form input[name="email"]', 'admin@crm.com');
    await page.fill('form input[name="password"]', 'admin');
    
    console.log('3. Clicking submit ...');
    await Promise.all([
      page.waitForNavigation(),
      page.click('form button[type="submit"]')
    ]);

    // 3. Verify Dashboard page load and seeded metrics
    console.log('4. Verifying Dashboard metrics ...');
    await expect(page.locator('h2:has-text("لوحة التحكم")')).toBeVisible();
    await expect(page.locator('text=إجمالي العملاء')).toBeVisible();
    await expect(page.locator('text=إجمالي الـ Leads')).toBeVisible();

    // Check seeded metrics count with specific card selectors to avoid strict mode violations
    await expect(page.locator('.amana-card:has-text("إجمالي العملاء") h2')).toHaveText('20');
    await expect(page.locator('.amana-card:has-text("إجمالي الـ Leads") h2')).toHaveText('50');

    // 4. Navigate to Customers management page
    console.log('5. Navigating to /customers ...');
    await page.goto('http://localhost:3009/customers');
    await expect(page.locator('h2:has-text("إدارة العملاء")')).toBeVisible();
    await expect(page.locator('.amana-table')).toBeVisible();

    // 5. Create a new Customer
    console.log('6. Creating a new customer ...');
    await page.click('text=إضافة عميل جديد');
    await expect(page.locator('input[name="name"]').first()).toBeVisible();

    // Fill in Create Form
    await page.fill('input[name="name"]', 'عميل تجريبي جديد');
    await page.fill('input[name="email"]', 'newtest@client.com');
    await page.fill('input[name="phone"]', '+966599999999');
    await page.fill('input[name="company"]', 'الشركة التجريبية');
    await page.fill('input[name="revenue"]', '150000');
    
    // Click submit in Create Modal
    console.log('7. Submitting create customer form ...');
    await page.click('button:has-text("Submit")');

    // Wait for the customer to appear in the table
    await expect(page.locator('text=عميل تجريبي جديد')).toBeVisible();
    console.log('Customer created successfully.');

    // 6. Update the newly created Customer
    console.log('8. Updating customer company name ...');
    // Click the Edit button in the row corresponding to our new customer
    // The edit button sets values in .edit-modal-wrapper form
    const row = page.locator('tr:has-text("عميل تجريبي جديد")');
    await row.locator('button:has-text("تعديل")').click();

    // Modify the company name inside the edit modal
    await expect(page.locator('.edit-modal-wrapper input[name="company"]')).toBeVisible();
    await page.fill('.edit-modal-wrapper input[name="company"]', 'الشركة التجريبية المعدلة');
    await page.fill('.edit-modal-wrapper input[name="revenue"]', '160000');
    await page.click('.edit-modal-wrapper button:has-text("Submit")');

    // Verify company name has been updated in the table
    await expect(page.locator('text=الشركة التجريبية المعدلة')).toBeVisible();
    console.log('Customer updated successfully.');

    // 7. Verify Customer Detail Page is empty of Leads
    console.log('9. Checking customer details (id = 21) ...');
    await page.goto('http://localhost:3009/customer_detail?id=21');
    await expect(page.locator('h2:has-text("تفاصيل العميل والـ Leads المرتبطة")')).toBeVisible();
    await expect(page.locator('text=عميل تجريبي جديد')).toBeVisible();
    await expect(page.locator('text=الشركة التجريبية المعدلة')).toBeVisible();
    await expect(page.locator('text=لا توجد فرص بيعية نشطة لهذا العميل.')).toBeVisible();

    // 8. Navigate to Leads and create a new Lead linked to Customer ID 21
    console.log('10. Creating a new lead linked to Customer 21 ...');
    await page.goto('http://localhost:3009/leads');
    await expect(page.locator('h2:has-text("إدارة الفرص البيعية (Leads)")')).toBeVisible();

    await page.click('text=إضافة Lead جديد');
    await page.fill('input[name="title"]', 'مشروع تجريبي للعميل الجديد');
    await page.fill('input[name="value"]', '75000');
    await page.fill('input[name="status"]', 'new');
    await page.fill('input[name="customer_id"]', '21');
    await page.click('button:has-text("Submit")');

    // Verify lead appears in leads table
    await expect(page.locator('text=مشروع تجريبي للعميل الجديد')).toBeVisible();
    console.log('Lead created successfully.');

    // 9. Go back to Customer Detail Page and check if Lead is displayed
    console.log('11. Re-checking customer details for linked Lead ...');
    await page.goto('http://localhost:3009/customer_detail?id=21');
    await expect(page.locator('text=مشروع تجريبي للعميل الجديد')).toBeVisible();
    await expect(page.locator('text=$75000')).toBeVisible();
    console.log('Relationship verification succeeded! Lead appears dynamically in Customer details.');

    // 10. Verify Reports Page loads and displays charts
    console.log('12. Checking Reports page ...');
    await page.goto('http://localhost:3009/reports');
    await expect(page.locator('h2:has-text("تقارير المبيعات والـ Leads")')).toBeVisible();
    await expect(page.locator('canvas[id^="chart_all_leads"]').first()).toBeVisible();
    console.log('All end-to-end CRM verification scenarios completed successfully!');
  });
});
