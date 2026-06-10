# Amana Live Compiler

هذا المجلد يحتوي أداة عملية لتحرير Amana ورؤية الناتج مباشرة في المتصفح.

الأداة ليست مترجماً منفصلاً. هي واجهة حول المترجم الحقيقي الموجود في جذر المشروع، وتستدعي:

```powershell
cargo run -- check ...
cargo run -- fmt ...
cargo run -- inspect-design ...
cargo run -- build ...
```

ثم تشغل تطبيق Node.js الناتج وتعرضه داخل iframe.

## التشغيل

من جذر المشروع:

```powershell
cd amana-live-compiler
npm run dev
```

افتح:

```text
http://127.0.0.1:4080
```

## الملفات

- `workspace/app.amana`: الملف الذي يحرره المتصفح.
- `.amana_live_dist/`: التطبيق الناتج بعد build.
- `server.js`: API الحفظ والفحص والبناء وتشغيل preview.
- `public/`: واجهة المحرر.

## الأوامر داخل الواجهة

- `Save`: يحفظ `workspace/app.amana`.
- `Check`: يشغل `amana check --json`.
- `Format`: يشغل `amana fmt --json` ويعيد تحميل النص المنسق.
- `Inspect`: يشغل `amana inspect-design --json`.
- `Build + Preview`: يشغل `amana build --json` ثم `node --check` ثم يشغل التطبيق الناتج.
- `Stop`: يوقف preview runtime.

## المنافذ

الافتراضي:

- Studio: `4080`
- Preview app: `4174`

تغييرها:

```powershell
$env:AMANA_STUDIO_PORT="4081"
$env:AMANA_PREVIEW_PORT="4175"
npm run dev
```

## تثبيت Node dependencies

عند أول build، الأداة تحاول تشغيل `npm install` داخل `.amana_live_dist` إذا لم تجد `node_modules`.

لتخطي ذلك واستخدام `NODE_PATH` من مجلدات build موجودة:

```powershell
$env:AMANA_LIVE_SKIP_INSTALL="1"
npm run dev
```

## اختصارات

- `Ctrl+S`: حفظ.
- `Ctrl+Enter`: build + preview.
- `Ctrl+Shift+F`: format.
- `Tab`: يضيف 4 مسافات داخل المحرر.

## ملاحظات

- Auto preview مفعّل افتراضياً مع debounce، ويمكن إغلاقه من أعلى الصفحة.
- preview يعمل عبر proxy على `/preview/` حتى يمكن عرضه داخل iframe.
- إذا حدث خطأ في Amana، ستراه كـ JSON diagnostics في اللوحة السفلية.
- preview proxy يزيل قيود iframe غير المناسبة ويعرض runtime الناتج مباشرة على `/preview/?t=...`.
- runtime الناتج يحتوي guards تمنع الفراغ الأفقي، قص النص العربي، وتضخم العناوين الثابتة داخل preview.
- API يحفظ ويبني عبر queue داخلي مع retry على أخطاء Windows المؤقتة مثل `UNKNOWN`, `EBUSY`, و`EPERM` عند فتح `workspace/app.amana`.
