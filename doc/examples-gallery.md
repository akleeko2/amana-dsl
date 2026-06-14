# Amana Examples Gallery

The `examples/` directory contains premium-designed showcases demonstrating the capabilities of the **Amana DSL v2 Design Engine**. These examples showcase advanced layout primitives, custom visual themes, typography scales, micro-animations, secure database mappings, and real-time form handlers.

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

# 6. Pro Dashboard (Enterprise Analytics Command Panel)
cargo run -- build examples/06_pro_dashboard.amana examples/06_pro_dashboard_dist

# 7. Nexus Portal (Asymmetric Workspace Feed Hub)
cargo run -- build examples/07_nexus_portal.amana examples/07_nexus_portal_dist

# 8. Atelier Aurelia (Timeless Luxury Horology Atelier)
cargo run -- build examples/08_atelier_aurelia.amana examples/08_atelier_aurelia_dist

# 9. Multi-File Portal (Modular landing page & Dark-mode Facebook clone)
cargo run -- build examples/09_multi_file_portal/main.amana examples/09_multi_file_portal_dist
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

### 4. Nova Creative (`04_nova_creative.amana`)
* **Theme & Voice:** Deep Orchid Neon Purple (`canvas: "#090514"`, `primary: "#d946ef"`, `accent: "#8b5cf6"` violet, `surface: glass`). Creative and bold voice.
* **Layout Blocks:**
  * `split` (hero asymmetric showcase showcasing design philosophies).
  * `grid` (portfolio rail showcasing visual works).
  * `stack` (client request questionnaire).
* **Key Features:**
  * Vibrant gradient blobs and orchid glow backdrops.
  * Lift-on-hover card transformations with smooth scale.
  * Database models: `ProjectBrief` (design client requests).

### 5. Cura Wellness (`05_cura_wellness.amana`)
* **Theme & Voice:** Modern Healing Teal Clean Mode (`canvas: "#f0fdf4"`, `primary: "#0f766e"`, `accent: "#14b8a6"`, `surface: elevated`). Soft, clean, and reliable voice.
* **Layout Blocks:**
  * `split` (clean hero section with appointment scheduler).
  * `grid` (doctor directory grid cards).
  * `stack` (clinical specialties list).
* **Key Features:**
  * High-density clean layout optimized for medical search.
  * Seamless reservation form connecting clients directly to caregivers.
  * Database models: `Appointment` (medical visit slots).

### 6. Pro Dashboard (`06_pro_dashboard.amana`)
* **Theme & Voice:** Professional Enterprise Blue/Teal Light Mode (`canvas: "#f0f2f5"`, `primary: "#1890ff"`, `accent: "#13c2c2"`, `surface: custom`). Corporate and analytical voice.
* **Layout Blocks:**
  * `column` (dashboard frame setup).
  * `grid` (analytics cards grid and KPI indicators).
* **Key Features:**
  * Interactive sidebar navigation supporting 7 tabs (Dashboard, Form, List, Profile, Result, Exception, Account).
  * Live Chart.js graphs (line chart for visits, bar chart for payments, doughnut chart for operation effects).
  * Database models: `ShopRanking` (sales leaderboards).

### 7. Nexus Portal (`07_nexus_portal.amana`)
* **Theme & Voice:** Deep Violet Slate Dark Mode (`canvas: "#0a0a0f"`, `primary: "#7c3aed"`, `accent: "#2563eb"`, `surface: elevated`). Modern digital workplace community feed.
* **Layout Blocks:**
  * `sidebar` (navigation menu layout).
  * `grid` (feed cards and trending boxes).
* **Key Features:**
  * Complex layout with profile cards, left sidebar shortcuts, main feed (supporting new posts creation), and active community members directory.
  * Interactive comments toggle for posts.
  * Database models: `NexusPost` (community announcements) and `Feedback` (user workspace metrics).

### 8. Atelier Aurelia (`08_atelier_aurelia.amana`)
* **Theme & Voice:** Luxury Timeless Horology Dark Mode (`canvas: "#0d0f12"`, `primary: "#d4af37"` gold, `accent: "#0b2e24"`, `surface: custom`). Sophisticated, high-end, premium craftsmanship tone.
* **Layout Blocks:**
  * `column` (main timeline).
  * `grid` (luxury specs details and bespoke configurator).
* **Key Features:**
  * Luxury watch bespoke configurator letting users toggle dial faces, straps, and casings to view real-time valuations.
  * Craftsmanship timeline showing mechanical blueprints, forging, assembly, and testing milestones in real-time.
  * Elegant dark-gold typography hierarchy with background glow effects.
  * Database models: `AtelierBooking` (private viewing reservations).

### 9. Multi-File Portal (`examples/09_multi_file_portal`)
* **Theme & Voice:** Modular Emerald-Gold Slate Dark Mode (`canvas: "#020617"`, `primary: "#10b981"`, `accent: "#1877f2"`, `surface: "glass-layered"`). Extremely professional development portal.
* **Key Features:**
  * **Multi-File Architecture**: Demonstrates module importing and code isolation across components (`section1.amana` through `section6.amana`), model declaration (`models.amana`), and routing entrypoint (`main.amana`).
  * **Interactive Tab Showcase**: Select between "The Heritage", "The Celestial", and "The Sovereign" specifications with Dynamic AlpineJS state switching.
  * **Facebook Clone (/facebook)**: Pixel-perfect Facebook dark-mode replica including left sidebar shortcut directories, middle story sliders (Create Story + 5 users), Roya News Donald Trump feed post fetched from SQLite models, sponsored ads, and birthdays notification.
  * Database models: `FacebookPost` (feed records) and `LeadBrief` (consultation form requests).
