# HTML Render v2, Components, and Forms

Amana compiles view declarations into highly optimized server-side templates (EJS) combined with Alpine.js client-side reactivity.

## Component Call Rules

Component calls must adhere to strict parser rules regarding children nodes:

### 1. Components Without Children
If a component does not contain nested elements, it **must** be written *without* a colon (`:`):

```amana
Navbar(brand: "Amana", sticky: true)
Footer()
```

### 2. Components With Children
If a component acts as a wrapper and contains children, it **must** use a colon (`:`):

```amana
Card(title: "Hello Dashboard"):
    p: "This is a card body child element."
```

*Note: The Amana compiler check stage detects violations and provides helpful correction diagnostics, while `amana fmt` automatically corrects formatting.*

---

## Standard Component Library

Amana provides a premium, responsive set of built-in components:

`Button`, `Card`, `Container`, `Section`, `Grid`, `Stack`, `Navbar`, `Hero`, `FeatureCard`, `PricingCard`, `FormField`, `Alert`, `Footer`, `Modal`, `Tabs`, `Badge`, `Kpi`, `Stat`, `LogoCloud`, `TestimonialCard`, `Timeline`, `TimelineItem`, `EmptyState`, `Split`, `Cluster`, `Sidebar`, `Icon`

---

## Navbar Component

The standard navigation bar supports branding, sticky layouts, and a clean, declarative routing list:

```amana
Navbar(brand: "Brand Name", sticky: true, links: "Home /, Features /#features, Contact /contact")
```

### Attributes:
- `brand` (string): Text brand name displayed on the left/right depending on locale direction.
- `sticky` (boolean): If `true`, the navigation bar is fixed to the top of the viewport with a frosted glass background (`backdrop-filter: blur(12px)`) and a subtle bottom border.
- `links` (string): A comma-separated list of links. Each link is formatted as `Label href` (separated by a space).
  - If no space is present and it starts with a `/`, the label and path are identical.
  - If no space is present and it doesn't start with a `/`, it defaults to `href="#"`.
  - Examples: `links: "الرئيسية /, عن_الشركة /about, اتصل_بنا /contact"`

---

## Forms and Actions (Forms v2)

Forms bind input inputs to database model controllers securely:

```amana
form [name, email, password]:
    connect User.register
    ui: card
    submit: "Create account"
    redirect success -> /

    field name:
        label: "Your Full Name"
        required: true
        placeholder: "Jane Doe"

    field email:
        label: "Email Address"
        type: email
        placeholder: "you@example.com"
        required: true

    field password:
        label: "Password"
        type: password
        help: "Must be at least 8 characters long."
        required: true
```

### Form Control Directives:
- `connect Model.action`: Maps server-side handlers (e.g. `.create`, `.update`, `.delete`, `.register`, `.login`).
- `ui`: Layout style (defaults to standard `card`).
- `submit`: Submission button text.
- `redirect success -> /`: Client redirect route path upon successful server validation.

### FormField Attributes:
- `name` (identifier): Corresponds to the model field name.
- `label` (string): Field label. Defaults to the field name if not specified.
- `placeholder` (string): Input helper placeholder text.
- `type` (type): HTML5 input type (e.g., `text`, `email`, `password`, `number`, `tel`).
- `required` (boolean/yes/1): When set to true, binds the `required` HTML5 attribute, appends an asterisk (`*`) inside an `.amana-required` marker, and sets `aria-required="true"` for screen-reader accessibility.
- `help` (string): Optional helper text rendered below the input inside a `<small class="amana-help">` node.

---

## Security: HTML Tag Restrictions

To prevent arbitrary script injection, layout breaking, or cross-site scripting (XSS) vulnerabilities, Amana enforces a strict allowlist at the semantic analysis stage.

Direct usage of raw lowercase HTML nodes that present potential security risks is blocked:

```amana
# The following raw HTML nodes will FAIL semantic validation:
script: "console.log('inject')"
iframe(src: "https://untrusted-site.com")
style: "* { display: none !important; }"
```

### Forbidden Tags List:
`script`, `iframe`, `object`, `embed`, `applet`, `link`, `meta`, `base`, `style`, `noscript`

### Remediation:
- Use **Amana components** (like `Icon`, `Button`, or layout grids) instead of custom markup.
- Declare visual configurations inside **design-grammar blocks** or custom `style:` sections of the view rather than raw style nodes.
