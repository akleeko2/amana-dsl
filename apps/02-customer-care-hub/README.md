# Customer Care Hub

A professional multi-file Amana example application demonstrating a full customer support platform.

## App Overview

**Customer Care Hub** is a dark-mode enterprise support platform with:

- **10+ views** across a full customer support workflow
- **7 models** with field validation, permissions, and foreign keys
- **5 custom components** with advanced scoped styles
- **Multi-file structure** with clean domain separation using imports
- **Full auth** with role-based access (Admin / Agent)

## File Structure

```
02-customer-care-hub/
├── app.amana              ← Entry point; imports all sub-files
├── config/
│   └── theme.amana        ← Dark theme, tokens, and global variants
├── models/
│   └── models.amana       ← All 7 data models with permissions
├── seeds/
│   └── seeds.amana        ← Realistic seed data
├── components/
│   └── components.amana   ← 5 custom UI components
├── routes/
│   └── routes.amana       ← All application routes
└── views/
    ├── home.amana         ← Public landing page
    ├── login.amana        ← Split-layout auth page
    ├── signup.amana       ← Registration with onboarding steps
    ├── dashboard.amana    ← Main command center
    ├── tickets.amana      ← Ticket management with modal
    ├── ticket_detail.amana ← Full ticket thread view
    ├── customers.amana    ← Customer CRM view
    ├── agents.amana       ← Team management & leaderboard
    ├── reports.amana      ← Analytics & performance reports
    └── settings.amana     ← Profile & preferences
```

## Models

| Model              | Purpose                                       |
|--------------------|-----------------------------------------------|
| `Agent`            | Support staff with roles and performance stats|
| `Ticket`           | Support requests with priority, SLA, channels |
| `TicketMessage`    | Threaded replies within a ticket              |
| `Customer`         | CRM records with CSAT and LTV tracking        |
| `KnowledgeArticle` | Self-service knowledge base articles          |
| `SatisfactionSurvey` | Post-resolution CSAT surveys               |
| `DailyMetric`      | Time-series operational metrics               |

## Build

```powershell
cargo run -- build apps/02-customer-care-hub/app.amana apps/02-customer-care-hub/dist
```

## Credentials

- **Admin**: `admin@carehub.dev` / `carepass1`
- **Agent**: `agent@carehub.dev` / `carepass1`
