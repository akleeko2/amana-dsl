# Live Preview

The live compiler app sits in `amana-live-compiler/` and drives the source/edit/build/preview loop.

## Flow

```text
edit workspace/app.amana
  -> build workspace/app.amana
  -> write runtime output
  -> start the generated Node app
  -> proxy /preview/ to the generated app
```

## Current behavior

- the workspace source file is compiled through the real compiler
- rebuilds follow source graph changes
- preview errors should be surfaced through the build API
- generated runtime checks can be validated with `node --check`

## Important notes

- preview output should not be treated as static HTML
- all layout and size issues still need to be validated in the browser
- the runtime now uses safer defaults for seeds, REST, HTTPS, and request error reporting

## Visual loop

Recommended AI loop:

```text
generate -> check --json -> fmt -> build -> preview -> screenshot -> adjust
```

