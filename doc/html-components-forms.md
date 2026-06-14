# HTML Rendering, UI Components, and Forms v2

Amana view rendering combines static HTML structure, custom and built-in CSS layout components, and a robust client-side state machine driven by **Alpine.js**.

---

## 📐 Component Call Rules

The Amana parser enforces two strict structural rules when invoking standard or custom UI components:

### 1. Components Without Nested Children
If a component does not wrap any child element, it **must** be written without a trailing colon (`:`):
```amana
Navbar(brand: "Aetheris Control", sticky: true)
Footer()
```

### 2. Components Wrapping Child Elements
If a component acts as a layout container containing child nodes, it **must** end with a colon (`:`) and indent all child elements underneath:
```amana
Card(title: "containment_status", subtitle: "Reactor core alpha"):
    p: "Containment field strength is stable at 98.7 T."
```

*Note: Violations are caught at compilation check time, and `amana fmt` automatically converts and cleans formatting.*

---

## 📚 Standard Components Directory

Amana includes 27 built-in components mapped directly to premium, RTL-supported, and responsive templates:

### 1. Layout & Shell Containers
- `Container(width: "wide" | "readable" | "standard")`: Restricts content to standard screen width grids.
- `Section(id: str)`: Wraps a major vertical page section with consistent padding.
- `Grid(min: str, gap: str)`: CSS Grid wrapper for bento boxes and responsive column collections.
- `Stack(gap: str)`: Aligns nested elements vertically.
- `Split(ratio: str = "1:1")`: Proportional two-column split layout.
- `Cluster(gap: str)`: Horizontally wraps inline elements (ideal for buttons and badges).
- `Sidebar(position: "left" | "right")`: Master-detail column split shell.

### 2. Presentation Cards & Elements
- `Card(title: str, subtitle: str, variant: "glass" | "elevated" | "luxury")`: Elegant container with border, hover shadows, and text headings.
- `FeatureCard(title: str, icon: str)`: Centered icon-text card for listing system capabilities.
- `PricingCard(plan: str, price: str, features: str)`: Subscription options card with action triggers.
- `TestimonialCard(author: str, role: str, avatar: str)`: Showcases reviews inside a blockquote element.
- `EmptyState(title: str, description: str, icon: str)`: Placeholder message when data views are empty.
- `LogoCloud(title: str)`: Flex-wrapped gray-scaled logo matrix container.

### 3. Timeline Components
- `Timeline(alternate: bool = false)`: Vertical line thread linking milestone points.
- `TimelineItem(title: str, timestamp: str, status: "success" | "warning" | "danger")`: Single node on the timeline thread containing telemetry details.

### 4. Elements & Interactive Controls
- `Button(label: str, href: str, variant: "solid" | "outline" | "ghost" | "link")`: Standard link/button controller.
- `Badge(label: str, variant: "primary" | "accent" | "success" | "warning" | "danger")`: Mini tag indicator for metadata.
- `Icon(name: str)`: Injects svg icons (supports Material Symbols and FontAwesome aliases).

### 5. Navigation & Framework Shells
- `Navbar(brand: str, sticky: bool, links: str)`: Header menu. `links` syntax: `"Home /, Features /#features, Contact /contact"`.
- `Footer(brand: str)`: Centered copyright footer layout.
- `Tabs(items: str, active: str)`: Navigation tab selector.
- `Modal(title: str, show: str)`: Floating layer popup toggled via client state variables.

### 6. Analytics & Telemetry Indicators
- `Kpi(title: str, value: str, trend: str, up: bool)`: Bento block displaying data metrics.
- `Stat(label: str, value: str)`: Clean metric description block.
- `FormField(name: str, label: str, placeholder: str, type: str, required: bool, help: str)`: Modular form input wrapper.
- `Alert(title: str, variant: "info" | "success" | "warning" | "error")`: Highlight box for system updates.

---

## ⚡ Client-Side Interactivity (Alpine.js Bindings)

Amana does not require custom JavaScript compilation. Interactivity is declared inline using binding keywords mapped directly to Alpine.js syntax at compiler codegen:

### 1. Event Listeners (`click:`)
Binds expression execution to user clicks:
```amana
button(click: "active_tab = 'monitor'"): "Reactor Core"
button(click: "show_modal = true"): "Launch Console"
```

### 2. Visibility Controls (`show:`)
Conditionally renders elements based on client-state evaluation:
```amana
div.alert(show: "coolant_valve < 25"):
    span: "CRITICAL: Increase Coolant flow!"
```

### 3. Text Interpolation (`text:`)
Injects reactive variables directly into elements:
```amana
span(text: coolant_valve): ""
span: "%"
```

### 4. Input State Binding (`model:`)
Maps slider or form input elements to client-state variables:
```amana
input.slider(type: "range", min: "0", max: "100", model: coolant_valve)
```

### 5. Reactive Classes (`class:`)
Toggles CSS classes dynamically:
```amana
a.sidebar-link(class: "{ 'active': current_tab == 'Dashboard' }", click: "current_tab = 'Dashboard'"):
    span: "Dashboard"
```

---

## 📝 Form v2 & Database Actions

Forms bind client-side input elements directly to server-side SQLite model actions securely.

### 1. Form Declaration Structure
```amana
form [email, msg]:
    connect Feedback.create
    ui: card
    submit: "Archive Anomaly Record"
    redirect success -> /dashboard
```
- `form [fields]`: List of field names bound to the form session.
- `connect Model.action`: Maps server handler hooks. Supported actions:
  - `Model.create`: Inserts a new row with submitted data.
  - `Model.update`: Modifies existing rows matching criteria.
  - `Model.delete`: Deletes rows matching identifiers.
  - `Model.register` / `Model.login`: Handles auth session generation.
- `ui`: Visual structure wrapper (e.g. `card`, `flat`, `outline`).
- `submit`: Submission button text.
- `redirect success -> /path`: Target redirect route on successful validation.

### 2. Validation Decorators
Inputs inside forms are automatically decorated by the schema:
- Fields marked `required` render with an asterisk (`*`) inside a `<span class="amana-required">` tag and bind `aria-required="true"`.
- If model constraint checks fail on the server, the field validation errors are flushed back to the client and highlighted next to input elements automatically.

---

## 🔒 Security: HTML Tag Restrictions

To maintain cross-site scripting (XSS) immunity and prevent template tampering, Amana restricts raw HTML tag declarations.

If any of the following tags are declared inside views, the compiler semantic check will fail:

`script`, `iframe`, `object`, `embed`, `applet`, `link`, `meta`, `base`, `style`, `noscript`

### How to resolve:
- **Scripts**: Implement state logic within the `client:` block and trigger transitions via standard `click:` attributes.
- **Styling**: Declare custom styles under the view's scoped `style:` block, or use utility classes instead of embedding raw `<style>` elements.
