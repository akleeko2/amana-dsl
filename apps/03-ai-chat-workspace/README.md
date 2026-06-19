# AI Chat Workspace Reference App

A professional, high-performance chat interface built entirely with Amana DSL. It features dynamic state transitions, responsive desktop/mobile layouts, markdown code styling, prompt catalogs, and mock conversation streams.

## Project Structure

```text
apps/03-ai-chat-workspace/
├── app.amana            # Entry point; imports all modules
├── config/
│   └── theme.amana      # Dark slate theme settings
├── models/
│   └── models.amana     # Database schemas (User, Conversation, Message, Prompt)
├── seeds/
│   └── seeds.amana      # Mock data seeding
├── routes/
│   └── routes.amana     # Routes and database query binds
└── views/
    ├── login.amana      # Center-aligned auth layout
    ├── chat.amana       # Scrollable message log, chips, and composer
    ├── prompts.amana    # Prompts list categorized via Tab & Accordion primitives
    └── settings.amana   # System preferences configuration
```

## Setup & Running

Build the Express/EJS target:

```powershell
cargo run -- build apps/03-ai-chat-workspace/app.amana apps/03-ai-chat-workspace/dist
```

Install packages and boot the Node server:

```powershell
cd apps/03-ai-chat-workspace/dist
npm install
$env:PORT="3103"
node app.js
```

Workspace Address:
```text
http://localhost:3103
```

## Demo Account Credentials

- **Email**: `user@chat.dev`
- **Password**: `chatpass1`
