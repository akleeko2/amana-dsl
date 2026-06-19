# Node Runtime And Security

Amana currently generates an Express/EJS/SQLite application. Runtime behavior is implemented in `src/codegen/express/*` and especially `src/codegen/express/static_files/engine.rs`.

## Generated Files

`amana build` writes:

```text
app.js
package.json
amana_ir.json
runtime/engine.js
middleware/security.js
middleware/hooks-worker.js
custom/hooks.js
views/<view>.ejs
views/login.ejs when the DSL does not define /login
```

Node package scripts:

```json
{
  "start": "node app.js",
  "dev": "nodemon app.js"
}
```

Runtime dependencies:

```text
express, express-session, sqlite3, ejs, argon2,
express-rate-limit, helmet
```

## Request Runtime

The generated runtime:

- Loads `amana_ir.json`.
- Opens the configured SQLite database.
- Creates/migrates model tables.
- Applies seed data according to environment policy.
- Registers API routes if `api.rest` is enabled.
- Registers route handlers from IR.
- Registers form submit routes from IR form actions.
- Renders EJS views with Alpine.js and runtime CSS.

## Sessions

Runtime sessions use `express-session`.

Production requirements:

- `SESSION_SECRET` must exist.
- The secret must not be weak.
- The secret must be at least 32 characters.
- Cookie `secure` is enabled in production.
- Cookies use `httpOnly` and `sameSite: "lax"`.

Development fallback:

- A development-only weak secret may be used outside production.

## CSRF Protection

The generated security middleware creates a per-session CSRF token.

Generated forms include:

```html
<input type="hidden" name="_csrf" value="<%= csrfToken %>">
```

POST requests are rejected when the submitted token does not match the session token.

## Password Handling

Fields with data type `password` are hashed with Argon2:

- seed insertion
- REST create/update
- form create/update/register
- default admin bootstrap behavior when applicable

Raw password submissions are not stored directly for password fields.

## Standard Libraries

Server fetches can call runtime standard libraries when capabilities allow them:

| Fetch target | Methods | Required capability |
| --- | --- | --- |
| `time` | `now` | `time` |
| `http` | `get`, `post` | `network.outbound` |
| `auth` | `verify`, `hash` | `auth` |

The compiler rejects standard library access inside render blocks. Runtime also checks capabilities before executing standard library fetches.

## REST API

REST routes are generated only when the app capability list includes:

```text
api.rest
```

Routes:

```text
GET    /api/<table>
GET    /api/<table>/:id
POST   /api/<table>
PUT    /api/<table>/:id
DELETE /api/<table>/:id
```

Access policy:

- If a model has `permit` rules, those rules are the authorization source of truth for that model.
- A model with `permit` rules is default-deny: an action must match an explicit rule by role, action, resource, optional row-level `where`, and optional field allowlist.
- `public` and `guest` rules can explicitly allow unauthenticated access.
- For models without `permit` rules, the legacy REST gate remains: `AMANA_ALLOW_PUBLIC_REST=true` allows public access, otherwise a configured `auth_model` requires a session user, and production public REST is forbidden without opt-in.
- REST list and server fetches filter rows through read policies before returning or rendering data.
- Read policies with `fields` apply field-level masking. Returned rows include `id` plus the fields listed by matching read rules.
- REST create/update enforce field allowlists when matching `permit` rules define `fields`.
- The policy runtime is covered by a multi-session integration test that logs in two principals, creates a row through a protected form, verifies REST row filtering, rejects cross-session REST updates, rejects cross-session form updates, and keeps the original row unchanged.

REST list pagination:

- `limit` defaults to 100.
- `limit` is clamped between 1 and 1000.
- `page` defaults to 1.
- `offset` overrides page-derived offset when supplied.

## Forms Runtime

Forms post to:

```text
/form-submit/<model>/<action>
```

Supported actions:

```text
create update delete login register logout
```

Runtime behavior:

- `login` loads user by email and verifies Argon2 password.
- `register` rejects duplicate email before inserting.
- `logout` destroys the session.
- `create` inserts accepted fields and resolved defaults.
- `update` updates accepted fields for matching `id` and resolved constraints.
- `delete` deletes matching `id` and resolved constraints.
- For models with `permit` rules, `create`, `update`, and `delete` also require matching runtime authorization. `update` and `delete` evaluate row-level policies against the existing row.
- For read rules, `fields` does not grant row access by itself; the role/action/resource/where policy must match first, then field masking is applied.
- `auth_model` controls the current-principal expression. Use `<auth_model>.current`; `User.current` remains compatible when `auth_model: User`.

When constraints prevent a row match, update/delete returns an authorization-style failure instead of silently changing unrelated rows.

## Seed Policy

Development:

- Seeds run automatically.

Production:

- Seeds are skipped by default.
- Set `AMANA_RUN_SEEDS=true` to run them explicitly.

Seed values are evaluated in a requestless context. The compiler rejects `User.current` in seeds.

## HTTPS And Production

In production:

- `trust proxy` is enabled.
- HTTP requests redirect to HTTPS when forwarded protocol is not `https`.
- Helmet is enabled.

## Hooks

Generated projects include:

```text
custom/hooks.js
middleware/hooks-worker.js
```

If `custom/hooks.js` already exists, codegen validates it before generating output.

The hook worker runs custom hook code inside a Node VM context with a timeout. Plugin/hook capability checks are enforced against app capabilities where runtime plugin manifests are involved.

## Static Security Checks Before Runtime

The compiler rejects:

- unsafe raw HTML tags in views
- unsafe CSS selectors and properties
- unsafe CSS/theme values
- missing standard library capabilities
- server-only standard library calls inside render blocks
- form defaults/constraints using current user outside protected views
- invalid query methods/columns

## Environment Variables

| Variable | Meaning |
| --- | --- |
| `NODE_ENV=production` | Enables production security behavior. |
| `SESSION_SECRET` | Required strong session signing secret in production. |
| `PORT` | Express listen port; default `3000`. |
| `AMANA_RUN_SEEDS=true` | Allows seed execution in production. |
| `AMANA_ALLOW_PUBLIC_REST=true` | Allows public REST access. |
| `AMANA_PLUGIN_KEY` | Runtime plugin key when plugin features are used. |

## Known Boundary

The generated Express path now uses the configured `auth_model` for current-principal expressions, default login table selection, route guards, form defaults/constraints, and authorization policy evaluation. The integration suite covers a non-`User` model (`Account`) across REST and form authorization paths. Keep using `<auth_model>.current` in source code; `User.current` is only the compatible spelling when `auth_model: User`.
