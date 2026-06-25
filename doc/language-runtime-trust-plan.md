# إصلاح عقد Amana بين اللغة والناتج

هذا المستند هو خطة الحوكمة والتنفيذ لسد فجوة: "Syntax تقبله اللغة لكنه لا يغيّر الناتج النهائي". القاعدة الحاكمة في Amana من الآن:

```text
أي Syntax عامة يجب أن تصل إلى:
parser -> AST -> semantic -> IR -> codegen -> runtime/docs/tests

وإلا تكون experimental/internal/rejected بوضوح، ولا توصف كميزة إنتاجية.
```

لا توجد حالة إنتاجية اسمها `partial`. كلمة `partial` يمكن أن تكون ملاحظة داخلية فقط، أما سطح اللغة العام فله حالة واحدة من:

- `implemented`: تعمل end-to-end ومغطاة باختبارات ووثائق.
- `experimental`: مكشوفة صراحة كتجريبية مع matrix يحدد أين تعمل وأين لا تعمل.
- `internal`: موجودة داخليًا ولا يجب تسويقها كـsyntax للمستخدم.
- `rejected`: يرفضها compiler بتشخيص واضح.

## الحالة التنفيذية الحالية

تم تنفيذ المسارات الحرجة التالية في الكود الحالي:

- `tokens:` تصل إلى IR وتتحول إلى CSS variables في صفحات EJS المولدة.
- `permit` يصل إلى IR ويُفرض في Express runtime على REST، وform mutations، وserver fetch filtering.
- `permit where` يعمل كـrow-level policy، و`fields` تعمل كـread masking في read policies وwrite allowlist في create/update policies.
- `auth_model` أصبح مصدر `<auth_model>.current` بدل افتراض ثابت لـ`User.current` في المسار الرئيسي للـruntime.
- `Chart(data, type, x, y)` أصبح له parser branch فعلي.
- ternary `cond ? then : else` أصبح مدعومًا في Pratt parser.
- `persist: memory/local/session/cookie` أصبح enum ويُنتج hydrate/watch في Alpine.
- `ResourceGrid/Table` ينتجان loading/error/empty/filter/sort behavior فوق البيانات المحملة من server fetch.
- `variants` تصل إلى IR وتتحول إلى CSS فعلي في EJS، وتُطبق عبر `variant` attributes أو `component: variant`.

## مصفوفة التغطية

هذه المصفوفة هي المرجع العملي لحالة الميزات. لا يجوز إضافة ميزة عامة للوثائق دون تحديثها.

| feature | public_status | parser | ast | semantic | ir | codegen | runtime_dev | runtime_prod | rest | forms | production_safe | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `tokens:` | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Emits CSS variables such as `--color-brand`, `--space-tight`, `--radius-panel`, `--shadow-card`. |
| `permit` | implemented | yes | yes | yes | yes | yes | yes | yes | yes | yes | yes | Enforces role/action/resource, row `where`, read field masking, and write `fields`; covered by multi-session REST/form integration tests. |
| `Chart` | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Parser supports `Chart(data, type, x, y)` with identifier arguments. Fully shapes Arabic text with RTL flow. |
| ternary | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Parser supports `cond ? then : else`. |
| `persist` | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | `memory/local/session/cookie` emit browser persistence behavior. |
| Resources loading/error/filter/sort | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Generated EJS includes lifecycle state, filters, and sort over server-fetched rows. |
| `auth_model` | implemented | yes | yes | yes | yes | yes | yes | yes | yes | yes | yes | Runtime uses `<auth_model>.current`; `User.current` remains compatible when `auth_model: User`. |
| variants | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Global and component-local variants emit target-specific CSS for base, hover, slots, and responsive rules. |
| Layout Components | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Emits Center, Cover, Reel, and Masonry markup and styling rules. |
| State & Feedback Systems | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Emits Skeleton shimmer, LoadingState, ErrorState, OfflineState, Toast, and Banner components. |
| Navigation Palette & Dropdown | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | n/a | yes | Emits Breadcrumb, Dropdown menu, and CommandPalette with focus traps. |
| Advanced Forms Interaction | implemented | yes | yes | yes | yes | yes | yes | yes | n/a | yes | FileUpload drag-and-drop, RichEditor editor, and ColorPicker inputs. |


### توضيح خلايا `n/a` في المصفوفة

عندما تقول المصفوفة `n/a` في عمود `rest` أو `forms`، فهذا لا يعني "غير مدعوم"، بل يعني **"لا ينطبق على هذه الميزة بحكم طبيعتها"**:

- `rest: n/a` و`forms: n/a` للميزات البصرية/اللغوية الصرفة (`tokens`, `Chart`, `ternary`, `persist`, `variants`, `resources`) لأنها لا تمر عبر بوابة تفويض REST أو إجراءات النماذج — هذه الميزات تعمل في كل البيئات لكنها ببساطة لا تملك مسار REST/forms لاختباره.
- إذا احتاجت ميزة بصرية يومًا إلى تأثير على التفويض، يجب فتح صف في المصفوفة بعمود `rest: yes`/`forms: yes` بدل `n/a`.

### رمز المصدر للقيم

`n/a` تعني **Not Applicable by design**، لا تعني **Not Available**. إذا لم تكن متأكدًا من الفرق، اقرأ القاعدة الحاكمة في أعلى المستند.

## Known Issues (مشاكل موثقة معروفة)

هذا القسم هو **مصدر الحقيقة الموحّد** لمشاكل المترجم المعروفة. تم نقله من `AMANA_VISUAL_LANGUAGE_AUDIT.md` ليكون ظاهرًا بجانب المصفوفة بدل إخفائه في ملف منفصل. هذه المشاكل تعني أن بعض المكونات القياسية **ليست production-safe بالكامل بعد** رغم أنها مذكورة في `language.md` كمدعومة.

> القاعدة: لا تصف ميزة هنا بأنها `implemented` في مصفوفة الميزات أعلاه إذا كانت مذكورة هنا كـbug نشط. يجب نقلها لـ`experimental` أو إضافة `*` يشير لهذا القسم.

### مشاكل بصرية/تخطيطية نشطة موثقة

لا توجد مشاكل تخطيطية نشطة معروفة حالياً. تم حل كافة المشكلات السابقة (تمدد البطاقات، دعم النوافذ المنبثقة Modals، التوافق مع الهواتف والأجهزة اللوحية، مشاكل اتجاه النصوص العربية في الرسومات وعناوين المكونات المتكررة).

| component / area | bug | impact | severity | workaround / status |
| --- | --- | --- | --- | --- |

### أنماط CSS خطرة مسموح بها حاليًا (compiler does not block)

الكمبايلر يسمح حاليًا بأنماط CSS قد تكسر التخطيط. هذه ليست أخطاء في حد ذاتها لكنها **سلوك محفوف بالمخاطر** لا يحذره الـcompiler:

| pattern | risk | recommended |
| --- | --- | --- |
| `min-height: 100vh` / `height: 100vh` inside `dashboard-shell` | Breaks container height, disables scrollbars. | Use `height: 100%`. |
| `overflow: hidden` on outer layout shells | Hides overflowing content, prevents scroll. | Avoid on outer shells. |
| `position: fixed` on custom components | Bypasses document flow, overlays elements. | Use layout canvas settings. |
| Hardcoded pixel `grid-template-columns` | Breaks mobile responsiveness. | Use `minmax()` or responsive rules. |

> خطط العمل: حظر `100vh` داخل `dashboard-shell` عبر تحذير في الـcompiler (محفوظ في `AMANA_VISUAL_LANGUAGE_AUDIT.md` قسم 5D كـFuture Product Enhancement).

### سياسة تحديث هذا القسم

- عند إصلاح أي bug هنا، **احذفه من الجدول** في نفس الـcommit وأضف اختبار regression.
- عند اكتشاف bug جديد، **أضفه هنا فورًا** ولا تتركه معلّقًا في ملف audit منفصل.
- لا يجوز أن يكون bug نشط في هذا القسم وفي نفس الوقت موصوفًا بـ`implemented`/`production_safe: yes` في مصفوفة الميزات أعلاه — اختر أحدهما.


## `permit` contract

`permit` ليس metadata. عند وجود أي policy على model، يصبح النموذج default-deny.

الصيغة المدعومة:

```amana
model Project:
    name: str
    status: str
    owner_id: int
    permit Manager read Project where owner_id = Account.current.id
    permit Manager update Project where owner_id = Account.current.id fields [status]
    permit Admin manage Project
```

قواعد التنفيذ:

- `role` يطابق `role`, `kind`, `type`, أو عناصر `roles` على principal الحالي.
- الأدوار الخاصة `public` و`guest` تسمح صراحة بالوصول غير المسجل.
- `authenticated` و`user` يطابقان أي principal مسجل.
- `action` يطابق action نفسه، أو `manage`، أو `*`.
- `resource` يطابق اسم model أو table name أو `*`.
- `where` يُقيّم على الصف الحالي وعلى request scope.
- `fields` في read policies تعمل كـfield-level read masking: الصف المسموح يبقى مرئيًا، لكن الناتج لا يعيد إلا `id` والحقول المذكورة.
- `fields` في create/update policies هي write allowlist؛ أي حقل خارجها يرفض.
- عند عدم وجود policies على model يبقى سلوك REST legacy gate كما هو لتجنب كسر التطبيقات القديمة.

نطاقات enforcement الحالية:

- REST:
  - `GET /api/<table>` يرشح الصفوف عبر read policy ثم يطبق read field masking.
  - `GET /api/<table>/:id` يرفض الصف إذا لم تطابق read policy، ويطبق read field masking على الصف المسموح.
  - `POST` يطبق create policy وfield allowlist.
  - `PUT` يقرأ الصف الحالي ثم يطبق update policy وfield allowlist.
  - `DELETE` يقرأ الصف الحالي ثم يطبق delete policy.
- Forms:
  - create/register يطبقان create policy وfield allowlist.
  - update/delete يطبقان policies على الصف الموجود، إضافة إلى `where` القديمة في form.
- Server fetches:
  - `all`, `filter`, و`find` تمر عبر read policy وتطبق read field masking عند وجود `fields`.
  - `count` يحسب عدد الصفوف التي تطابق read policy، ولا يعرض حقولًا.

## `variants` contract

`variant` لم يعد metadata. كل variant عام أو محلي يصل إلى IR ثم يولد CSS داخل صفحات EJS.

الصيغ المدعومة:

```amana
variant Card.glass:
    base:
        background: glass
        radius: soft
    hover:
        shadow: floating
    slots:
        title:
            color: accent
    responsive:
        mobile:
            padding: sm

component CardShell:
    variants:
        compact:
            base:
                padding: sm
```

قواعد التطبيق:

- target يجب أن يكون standard component معروفًا أو custom component مصرحًا به.
- standard components تطبق variant عبر attribute مثل `Card(variant: "glass")` أو nested design block مثل `component: variant: glass`.
- custom components تطبق variant بعد inlining عبر class ثابت `amana-component-<name>` مع `amana-variant-<variant>`.
- `base`, `hover`, `slots`, و`responsive` تتحول إلى CSS فعلي. breakpoints المدعومة هي `desktop`, `tablet`, `mobile`.
- قيم CSS تمر عبر نفس تحويل CSS DSL المستخدم في `style:`، لذلك تعمل tokens مثل `background: glass`, `radius: soft`, و`shadow: floating`.

## `auth_model` contract

المصدر الرسمي للمستخدم الحالي هو:

```amana
<auth_model>.current
```

مثال:

```amana
app TrustRuntime:
    auth_model: Account

view Home:
    protected:
        allow: Account.current != null
        deny: -> /login
        unauthenticated: -> /login
```

قواعد التنفيذ:

- semantic analyzer يربط `Account.current` بالموديل المحدد في `auth_model`.
- EJS/codegen يعيد كتابة `<auth_model>.current` إلى `currentUser` في runtime.
- route guards وform defaults/constraints وpolicy `where` تستخدم نفس principal.
- default login route يستخدم جدول `auth_model` بدل hardcode إلى `user`.
- `User.current` يبقى متوافقًا فقط عندما يكون `auth_model: User`.

## Governance

### Definition Of Done لأي Syntax عام

أي إضافة للغة يجب أن تثبت:

- parser test.
- AST representation.
- semantic validation.
- IR representation أو قرار صريح أنها compile-time only.
- codegen/runtime effect.
- positive and negative tests.
- documentation update.
- تحديث coverage matrix.

### CI/Docs Gate

المطلوب قبل دمج أي تغيير لغة:

```powershell
cargo test
python scripts/language_inventory.py --check
scripts/search-language.ps1 -Area all
```

لا تعتمد الاختبارات الجديدة على examples القديمة كمصدر حقيقة. examples مرجع مساعد فقط، والحقيقة تأتي من parser/semantic/IR/runtime tests.

## المرحلة الأولى: الأمان والثقة

تم تنفيذ عناصر المرحلة الأولى الأساسية في هذا التغيير:

- `permit` end-to-end.
- `auth_model` runtime generalization.
- `tokens` IR/CSS emission.
- `persist` browser persistence.
- `Chart` parser syntax.
- ternary parser support.
- Resource lifecycle runtime behavior.
- `variants` IR/CSS/runtime application.
- اختبارات مركزة تثبت وصول هذه الميزات إلى parser/IR/generated runtime.
- اختبار تكاملي متعدد الجلسات يثبت `permit` مع `auth_model` غير `User` عبر REST وforms: المالك ينشئ ويقرأ، والجلسة الأخرى لا ترى الصف ولا تستطيع تعديله.

المتبقي لتعزيز المرحلة الأولى:

- لا توجد حالة `partial` متبقية في سطح الإنتاج لهذه المرحلة. أي مسار جديد في REST/forms يجب أن يضيف اختبار جلسات قبل تغيير المصفوفة.

## المرحلة الثانية: إكمال السطح العام

الأولوية المتبقية:

1. إضافة LSP references وrename فوق فهرس الرموز الحالي.
2. توسيع completion الدلالي إلى حقول النماذج العامة عندما يحدد السياق المالك بوضوح.
3. إنشاء certified examples صغيرة تعمل في CI ولا تستخدم Syntax غير production-safe.
