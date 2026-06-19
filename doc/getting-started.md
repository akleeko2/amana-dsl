# Getting Started With Amana / البداية مع Amana

> **Audience:** someone who wants to *use* Amana to build a web app — not someone working *on* the compiler. If you are contributing to the compiler itself, start with [README.md](language.md) and [language-inventory.generated.md](language-inventory.generated.md) instead.
>
> **الجمهور:** من يريد *استخدام* Amana لبناء تطبيق ويب — وليس من يعمل *على* المترجم نفسه.

## What Is Amana? / ما هي Amana؟

Amana is a **declarative domain-specific language** (`.amana` files). You describe your data models, routes, and views in one file (or a small graph of files), and the Amana compiler generates a complete, runnable **Node.js + Express + EJS + SQLite** web application for you.

Amana هي **لغة وصفية** (ملفات `.amana`). تصف نماذج بياناتك ومساراتك وواجهاتك في ملف واحد، فيُولّد لك المترجم تطبيق ويب كاملًا بـ Node.js + Express + EJS + SQLite جاهز للتشغيل.

### The Mental Model / النموذج الذهني

```
amana source (.amana)
        │
        ▼
  ┌───────────┐
  │ Amana CLI │   cargo run -- build app.amana dist
  └───────────┘
        │
        ▼
Node/Express/EJS/SQLite app
        │
        ▼
  http://localhost:3000
```

Three building blocks cover ~90% of any Amana app:

| Block | Purpose / الغرض |
| --- | --- |
| `model` | A database table + its fields. جدول قاعدة بيانات وحقوله. |
| `route` | A URL path mapped to a view. مسار URL مرتبط بواجهة. |
| `view` | An HTML page: protected access, server fetches, client state, render tree, styles. صفحة HTML: صلاحيات، جلب بيانات، حالة عميل، شجرة عرض، أنماط. |

## Prerequisites / المتطلبات

1. **Rust toolchain** — to compile Amana itself. Install from <https://rustup.rs>. Needed only once.
   - سلسلة أدوات Rust لتجميع Amana نفسها.
2. **Node.js 18+ and npm** — to run the generated app.
   - Node.js و npm لتشغيل التطبيق المولّد.
3. **Python 3** — only needed to regenerate the language inventory (`scripts/language_inventory.py`). Optional for app authors.
   - Python 3 — لتحديث فهرس اللغة فقط. اختياري لكتّاب التطبيقات.

Verify:

```powershell
cargo --version
node --version
npm --version
```

## Step 1 — Build The Compiler / بناء المترجم

From the repository root:

```powershell
cargo build --release
```

The binary is `target\release\amana.exe` (Windows) or `target/release/amana` (Unix). During development you can skip the release build and prefix every command with `cargo run --`.

## Step 2 — Your First App / تطبيقك الأول

Create `app.amana`:

```amana
app HelloAmana:
    title: "Hello Amana"
    db_path: "hello.db"

model User:
    email: email unique required
    password: password required min 8
    role: str default "member"

route / -> view Home

view Home:
    canvas:
        layout: column
        density: comfortable
    render:
        div.page:
            h1: "مرحبا بك في Amana"
            p: "Hello from your first Amana app."
```

### Step 3 — Check It / تحقق منه

```powershell
amana check app.amana --json
```

Expected: `"ok": true`. If you see an error, the JSON includes `stage`, `line`, `column`, `message`, and optional `suggestion`.

### Step 4 — Build And Run / ابنِ وشغّل

```powershell
amana build app.amana dist
cd dist
npm install
npm run dev
```

Open <http://localhost:3000> in your browser. You should see your heading.

> For live rebuilds while you edit `app.amana`, use `amana dev app.amana dist` instead — it watches the source graph and rebuilds automatically. See [cli-dx.md → Dev Server](cli-dx.md#dev-server-and-live-rebuild).

## Step 5 — Add Auth And Data / أضف مصادقة وبيانات

This is where Amana shines. Extend `app.amana`:

```amana
app NotesApp:
    title: "Notes"
    auth_model: User
    capabilities:
        - auth

model User:
    email: email unique required
    password: password required min 8

model Note:
    title: str required max 120
    body: str
    owner_id: int foreign_key User(id) on_delete CASCADE

seed User:
    email: "demo@example.com"
    password: "password123"

route /notes -> view Notes

view Notes:
    protected:
        allow: User.current != null
        deny: -> /login
        unauthenticated: -> /login
    server:
        fetch notes = Note.filter(owner_id: User.current.id, limit: 50)
    client:
        state show_form = false
    render:
        div.page:
            h1: "My Notes"
            Button(label: "New note", click: show_form = true)
            div(show: show_form):
                form [title, body]:
                    connect Note.create
                    default owner_id = User.current.id
                    submit: "Save"
                    redirect success -> /notes
                    field title:
                        label: "Title"
                        required: true
                    field body:
                        label: "Body"
                        type: textarea
            for note in notes:
                article.card:
                    h3: note.title
                    p: note.body
```

Key things that just happened, automatically:

- **Argon2 password hashing** on the `password` field. تجزئة كلمات المرور تلقائيًا.
- **CSRF protection** injected into the form. حماية CSRF تلقائية.
- **A `/login` view** auto-generated because none was defined (the protected view redirects there). واجهة `/login` تُولّد تلقائيًا.
- **Row ownership** enforced: `default owner_id = User.current.id` + the protected view means each user only sees their notes. ملكية الصفوف مضمونة.
- **A seed user** created on first run (dev only). مستخدم أولي يُنشأ في التطوير.

Log in with `demo@example.com` / `password123`.

## Where To Go Next / الخطوات التالية

| You want to… | Read |
| --- | --- |
| Learn the full language surface | [language.md](language.md) |
| See every component and form option | [html-components-forms.md](html-components-forms.md) |
| Style, theme, RTL, design grammar | [css-theme-rtl.md](css-theme-rtl.md) |
| Understand security, sessions, REST | [node-runtime-security.md](node-runtime-security.md) |
| Multi-file projects and editor support | [multi-file-lsp.md](multi-file-lsp.md) |
| Avoid common design mistakes | [design-anti-patterns.md](design-anti-patterns.md) |
| Run a complete real app | the `apps/` directory, e.g. `apps/02-customer-care-hub/` |
| See what works vs. known bugs | [language-runtime-trust-plan.md → Known Issues](language-runtime-trust-plan.md) |

## Troubleshooting / استكشاف الأخطاء

**`cargo build` fails** — Make sure you have a recent Rust toolchain (`rustup update`). Amana has no exotic dependencies.

**`npm install` fails in `dist/`** — Ensure Node.js 18+ and that you are online for the first install (express, sqlite3, argon2, etc.).

**`ReferenceError` in the browser** — You probably used a client-state variable inside an EJS tag. See [design-anti-patterns.md → Alpine inside EJS](design-anti-patterns.md#3-alpine-js-variables-inside-ejs-tags--enforced--مُلزَم).

**Page looks broken on mobile** — Add `responsive: mobile.columns: 1` to your grids. See [design-anti-patterns.md → Mobile Responsive](design-anti-patterns.md#2-missing-mobile-responsive-rules--active-risk--خطر-نشط).

**A component behaves wrong** — Check whether it is a documented Known Issue in [language-runtime-trust-plan.md](language-runtime-trust-plan.md) before assuming it is your code.

**`User.current` errors in a form** — The view must be `protected:`. See [design-anti-patterns.md → Protect Current User Writes](design-anti-patterns.md#6-unprotected-current-user-writes--enforced--مُلزَم).
