# Implementation Plan: Premium Amana v2 Examples Rewrite

We will replace the existing 10 examples with 5 brand-new, extremely high-end example files that demonstrate the full range of the Amana v2 Design Engine's layout primitives, CSS variables, micro-animations, and form actions.

## Proposed Changes

### Deletion of Old Examples
We will delete all 10 old examples in the `examples/` directory to clean up the workspace:
- `01_saas_landing.amana`
- `02_editorial_story.amana`
- `03_developer_console.amana`
- `04_agency_portfolio.amana`
- `05_marketplace_grid.amana`
- `06_course_cohort.amana`
- `07_event_summit.amana`
- `08_dashboard_launch.amana`
- `09_medical_booking.amana`
- `10_creative_studio.amana`

### New Examples Specifications
We will create 5 premium-designed examples in the `examples/` directory. Each example will feature:
1. A unique curated colorway/theme (HSL, glassmorphism, luxury day, high-contrast, etc.).
2. At least 4 distinct sections demonstrating layout primitives.
3. Database model schemas, seed data, and client-side states.
4. Custom inline styles, micro-animations (`hover: lift-glow`, `hover: scale`, `stagger-up`), and responsive column rules.

---

### [Component Name] Examples

#### [DELETE] [01_saas_landing.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/01_saas_landing.amana)
#### [DELETE] [02_editorial_story.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/02_editorial_story.amana)
#### [DELETE] [03_developer_console.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/03_developer_console.amana)
#### [DELETE] [04_agency_portfolio.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/04_agency_portfolio.amana)
#### [DELETE] [05_marketplace_grid.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/05_marketplace_grid.amana)
#### [DELETE] [06_course_cohort.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/06_course_cohort.amana)
#### [DELETE] [07_event_summit.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/07_event_summit.amana)
#### [DELETE] [08_dashboard_launch.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/08_dashboard_launch.amana)
#### [DELETE] [09_medical_booking.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/09_medical_booking.amana)
#### [DELETE] [10_creative_studio.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/10_creative_studio.amana)

#### [NEW] [01_saas_aura.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/01_saas_aura.amana)
* **Title:** SaasAura Analytics Control Panel
* **Aesthetic:** Dark Neon Space Glassmorphism.
* **Layouts Used:** `split` (Hero), `bento` (Analytics Metrics), `asymmetric` (Feature Showcase), `stack`/`grid` (Feedback).
* **Models:** `Metric` (kpi_name, value, trend), `Feedback` (email, msg).
* **Sections:**
  1. Hero section: Indigo & Cyan mesh gradient layout split.
  2. Metrics Grid: Bento layout demonstrating 6-column bento auto-placements.
  3. Core Capabilities: Asymmetric visual card showing a live-updating system readout.
  4. Connect with Us: Glass-morphic feedback form.

#### [NEW] [02_maison_luxe.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/02_maison_luxe.amana)
* **Title:** MaisonLuxe Luxury Atelier
* **Aesthetic:** High-end Day Mode Editorial (Gold and Deep Emerald). Serif typography.
* **Layouts Used:** `split-diagonal` (Hero), `magazine` (Atelier Catalogue), `editorial` (Heritage Split), `grid` (Reservation Booking).
* **Models:** `CollectionItem` (title, description, price, season), `Booking` (name, email, date).
* **Sections:**
  1. Hero: Split-diagonal visual layout with diagonal cut shapes.
  2. Collection Catalogue: 12-column magazine layout demonstrating headline span cards.
  3. Heritage: Editorial split showing quote cards.
  4. Consultation Booking: Premium minimal form card.

#### [NEW] [03_vortex_console.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/03_vortex_console.amana)
* **Title:** Vortex Console Room
* **Aesthetic:** High-Tech Obsidian Console (Neon Lime & Emerald). Technical/blueprint grid texture.
* **Layouts Used:** `command-center` (Hero Terminal), `timeline` (Deployment Stages), `masonry` (Diagnostics Logs), `stack` (Access Signup).
* **Models:** `DeployLog` (stage, duration, status, details).
* **Sections:**
  1. Hero: Command-center layout displaying DevOps terminal control panels.
  2. Deployment Flow: Vertical alternate timeline displaying compilation stages.
  3. Diagnostics: Masonry grid showcasing log metrics and KPIs.
  4. Console Access: Sleek signup form with custom styling.

#### [NEW] [04_nova_creative.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/04_nova_creative.amana)
* **Title:** Nova Creative Studio
* **Aesthetic:** Deep Orchid Dark Mode (Hot Pink & Royal Blue). Soft blob visual shapes.
* **Layouts Used:** `asymmetric` (Hero), `showcase-rail` (Creative Portfolio), `split` (Studio Ethos), `grid` (Brief Submission).
* **Models:** `ProjectBrief` (client, email, brief, budget).
* **Sections:**
  1. Hero: Asymmetric layout with stagger-up motion entrance and hover lift-glow.
  2. Portfolio Showcase: Showcase-rail layout demonstrating project details.
  3. Ethos & Principles: Split layout presenting card items.
  4. Start a Project: Interactive form brief card.

#### [NEW] [05_cura_wellness.amana](file:///c:/Users/Lenovo/Downloads/مشروع%20لغة%20برمجة/examples/05_cura_wellness.amana)
* **Title:** Cura Wellness Platform
* **Aesthetic:** Modern Healing Day Mode (Teal & Mint Emerald). Soft rounded shapes.
* **Layouts Used:** `split` (Hero), `sidebar` (Specialties Navigation), `grid` (Doctor Bios), `stack` (Appointment Scheduling).
* **Models:** `Appointment` (patient_name, email, department, appointment_date).
* **Sections:**
  1. Hero: Split layout with doctor stats checklist.
  2. Medical specialties: Sticky sidebar navigation demonstrating layout: sidebar.
  3. Doctor Directory: Responsive cards showing specialists bios.
  4. Book Appointment: Direct appointment scheduling form card.

---

## Verification Plan

### Automated Tests
1. We will compile all examples using our compiler dev binary:
   `cargo run -- build examples/01_saas_aura.amana .amana_verify_dist/01_saas_aura`
   etc.
2. We will run our verification script:
   `powershell -File C:\Users\Lenovo\.gemini\antigravity\brain\485c309b-8c61-4267-80cf-8c1243d14b60\scratch\verify-all-examples.ps1`
   This will run `node --check` syntax verification and EJS tag validation on all generated files to ensure no unclosed EJS tags or scoping syntax issues exist.
3. We will run `cargo test` to verify the compiler code remains fully green.
