# Multi-file Imports and LSP

## Multi-file imports

Amana supports source graphs through:

```amana
import "./models/user.amana"
import "./views/home.amana"
```

Imports are resolved relative to the importing file.

## Source graph behavior

- `amana check` resolves the full graph
- `amana build` compiles the full graph
- `amana fmt --all` can format the graph from one entry file
- `amana dev` watches the graph for rebuilds

## LSP

The current LSP provides a basic stdio server with diagnostics, completion, and formatting hooks. It is intentionally small and tied to the same compiler pipeline.

## What still matters

- diagnostics should include `file_path`
- completion should stay aligned with the source graph
- formatter and checker should remain deterministic across files

