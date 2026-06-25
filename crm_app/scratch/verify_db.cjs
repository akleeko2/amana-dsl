const sqlite3 = require('c:/Users/Lenovo/Downloads/مشروع لغة برمجة/crm_app/dist/node_modules/sqlite3').verbose();
const db = new sqlite3.Database('c:/Users/Lenovo/Downloads/مشروع لغة برمجة/crm_app/dist/crm.db');

db.serialize(() => {
  db.get('SELECT COUNT(*) AS count FROM customer', (err, row) => {
    if (err) {
      console.error('Error querying customer count:', err);
    } else {
      console.log('Customer Count:', row.count);
    }
  });

  db.get('SELECT COUNT(*) AS count FROM lead', (err, row) => {
    if (err) {
      console.error('Error querying lead count:', err);
    } else {
      console.log('Lead Count:', row.count);
    }
  });

  db.get('SELECT COUNT(*) AS count FROM user', (err, row) => {
    if (err) {
      console.error('Error querying user count:', err);
    } else {
      console.log('User Count:', row.count);
    }
  });
});

db.close();
