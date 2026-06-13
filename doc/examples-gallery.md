# Amana Examples Gallery

The `examples/` directory contains five premium-designed showcases demonstrating the capabilities of the **Amana DSL v2 Design Engine**. These examples showcase advanced layout primitives, custom visual themes, typography scales, micro-animations, secure database mappings, and real-time form handlers.

---

## 🚀 Build & Verification Commands

All examples are verified to compile successfully without warnings and produce fully valid Node.js/Express runtimes with EJS views. You can compile them using the following commands:

```powershell
# 1. SaaS Aura (AI Analytics Dashboard)
cargo run -- build examples/01_saas_aura.amana examples/01_saas_aura_dist

# 2. Maison Luxe (Luxury Editorial Atelier)
cargo run -- build examples/02_maison_luxe.amana examples/02_maison_luxe_dist

# 3. Vortex Console (DevOps Cyber Telemetry Center)
cargo run -- build examples/03_vortex_console.amana examples/03_vortex_console_dist

# 4. Nova Creative (Design Studio & Showcase)
cargo run -- build examples/04_nova_creative.amana examples/04_nova_creative_dist

# 5. Cura Wellness (Teal Health & Wellness Directory)
cargo run -- build examples/05_cura_wellness.amana examples/05_cura_wellness_dist
```

---

## 🎨 Premium Examples Showcase

### 1. SaaS Aura (`01_saas_aura.amana`)
* **Theme & Voice:** Cyberpunk/Space Control Center (`canvas: "#05070f"`, `primary: "#00f0ff"` glowing cyan, `accent: "#ff007f"` neon magenta, `font_family: "Space Mono"`). Strict technical command room voice.
* **Layout Blocks:**
  * `column` (main container stack).
  * `grid` (Bento diagnostic panels and sliders).
  * `flex` (interactive control panels and particle colliders).
* **Key Features:**
  * Interactive Coolant and Magnetic Containment sliders that dynamically update status banners (SAFE/WARNING/DANGER) based on thresholds.
  * Built-in Particle Accelerator simulator to compute simulated fusion yields based on particle choices and excitation states.
  * System Anomaly Registry with a database form connected directly to SQLite `Feedback.create` and live telemetry log archives.
  * Database models: `Metric` (reactor metrics) and `Feedback` (telemetry anomaly logs).

### 2. Maison Luxe (`02_maison_luxe.amana`)
* **Theme & Voice:** Light Mode Luxury Day Editorial (`canvas: "#fcfbf7"`, `primary: "#111111"`, `accent: "#d4af37"` gold, `font_family: "Playfair Display"` serif). Elegant and architectural voice.
* **Layout Blocks:**
  * `split` (hero showcase with high-contrast text and boutique reservation entry).
  * `grid` (catalog sections with gold borders and custom metadata badges).
  * `stack` (heritage timeline and reservation flow).
* **Key Features:**
  * Floating transparent header menu with custom luxury styling.
  * Clean bottom-border input fields for scheduling atelier bookings.
  * Serif typography-driven visual hierarchy.
  * Database models: `CollectionItem` (seasonal fashion lines) and `Booking` (atelier sessions).

### 3. Vortex Console (`03_vortex_console.amana`)
* **Theme & Voice:** Obsidian Hacker Cyberpunk (`canvas: "#020813"`, `primary: "#10b981"` emerald, `accent: "#34d399"` neon mint, `font_family: "Space Mono"` / `"Fira Code"`). Dev-ops technical command style.
* **Layout Blocks:**
  * `split` (active DevOps stage telemetry header).
  * `grid` (bento diagnostic reading cards with glowing borders).
  * `stack` (stage build timelines and diagnostic command prompt input).
* **Key Features:**
  * Alternate stage timeline with monospace indicators.
  * Interactive CLI input simulator displaying stdout/stderr logs.
  * Custom code success badges (`.code.success`) with glowing shadows.
  * Database models: `DeployLog` (stage log lines) and `AccessRequest` (developer telemetry logs).

### 5. Nova Creative (`04_nova_creative.amana`)
* **Theme & Voice:** Deep Orchid Neon Purple (`canvas: "#090514"`, `primary: "#d946ef"`, `accent: "#8b5cf6"` violet, `surface: glass`). Creative and bold voice.
* **Layout Blocks:**
  * `split` (hero asymmetric showcase showcasing design philosophies).
  * `grid` (portfolio rail showcasing visual works).
  * `stack` (client request questionnaire).
* **Key Features:**
  * Vibrant gradient blobs and orchid glow backdrops.
  * Lift-on-hover card transformations with smooth scale.
  * Database models: `ProjectBrief` (design client requests).

### 6. Cura Wellness (`05_cura_wellness.amana`)
* **Theme & Voice:** Modern Healing Teal Clean Mode (`canvas: "#f0fdf4"`, `primary: "#0f766e"`, `accent: "#14b8a6"`, `surface: elevated`). Soft, clean, and reliable voice.
* **Layout Blocks:**
  * `split` (clean hero section with appointment scheduler).
  * `grid` (doctor directory grid cards).
  * `stack` (clinical specialties list).
* **Key Features:**
  * High-density clean layout optimized for medical search.
  * Seamless reservation form connecting clients directly to caregivers.
  * Database models: `Appointment` (medical visit slots).
