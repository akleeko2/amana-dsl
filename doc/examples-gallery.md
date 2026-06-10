# Examples Gallery

The `examples/` folder contains 5 premium-designed examples showcasing the advanced v2 Design Engine's layout primitives, visual themes, custom typography, micro-animations, and database integrations.

## Verification

Each example compiles successfully and passes node syntax and EJS tag validation checks:

```powershell
cargo run -- build examples/01_saas_aura.amana .amana_verify_dist/01_saas_aura
```

## Index

| Example | Title & Theme | Layout Primitives | Models & Actions |
| --- | --- | --- | --- |
| `examples/01_saas_aura.amana` | **SaasAura Analytics Control Room**<br>Dark Neon Space Glassmorphism | `split` (Hero), `bento` (Operations Board), `asymmetric` (Architecture) | `Metric`, `Feedback` |
| `examples/02_maison_luxe.amana` | **MaisonLuxe Luxury Atelier**<br>Light Mode Editorial Day Mode (Serif typography) | `split-diagonal` (Hero), `magazine` (Catalogue), `editorial` (Heritage) | `CollectionItem`, `Booking` |
| `examples/03_vortex_console.amana` | **Vortex Console Room**<br>Obsidian High-Tech Developer (Lime/Emerald) | `command-center` (Hero Terminal), `masonry` (Diagnostics), `stack` | `DeployLog`, `AccessRequest` |
| `examples/04_nova_creative.amana` | **Nova Creative Studio**<br>Deep Orchid Motion Theme (Pink/Blue Blobs) | `asymmetric` (Hero), `showcase-rail` (Portfolio), `split` (Ethos) | `ProjectBrief` |
| `examples/05_cura_wellness.amana` | **Cura Wellness Platform**<br>Modern Day Healing Theme (Teal/Mint Emerald) | `split` (Hero), `sidebar` (Specialties), `bento` (Doctor Directory) | `Appointment` |

## Why These Matter

- **Visual Diversity:** Ranging from premium luxury day-mode editorial layout to high-contrast developer obsidian blueprint terminal.
- **Layout Sophistication:** Utilizes `bento` auto-placements, `showcase-rail` horizontal flows, `magazine` spanning grids, `sidebar` sticky columns, and `split-diagonal` cutouts.
- **Data Integrations:** Connects front-end inputs (e.g. feedback, scheduling, access signup) directly to database models with secure handlers.
- **Micro-Animations & Visuals:** Features hover states (`lift-glow`, `scale`), entrance transitions (`stagger-up`), gradient accents (`spotlight`, `aurora`), and layered borders.
