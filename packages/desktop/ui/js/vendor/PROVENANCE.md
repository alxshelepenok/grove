# Vendored JavaScript provenance

Every file in this directory is a verbatim upstream artifact, recorded here so the bytes can be re-verified against the source at any time. Verification:

```bash
sha256sum packages/desktop/ui/js/vendor/<file>
```

must match the SHA-256 below, and the bytes must be identical to the upstream download.

| File | Package | Version | Upstream URL | SHA-256 |
| --- | --- | --- | --- | --- |
| `d3.js` | [d3](https://www.npmjs.com/package/d3) (ISC license) | 7.9.0 | `https://cdn.jsdelivr.net/npm/d3@7.9.0/dist/d3.min.js` (identical bytes at `https://unpkg.com/d3@7.9.0/dist/d3.min.js`) | `f2094bbf6141b359722c4fe454eb6c4b0f0e42cc10cc7af921fc158fceb86539` |
| `rx.js` | [rxjs](https://www.npmjs.com/package/rxjs) (Apache-2.0 license) | 7.8.2 | `https://unpkg.com/rxjs@7.8.2/dist/bundles/rxjs.umd.min.js` (identical bytes at `https://cdn.jsdelivr.net/npm/rxjs@7.8.2/dist/bundles/rxjs.umd.min.js`) | `2152e8a794982170a4c1dae32a74e31a81218fd74781c27b0d628a02bf532413` |

Notes:

- `d3.js` is the unmodified minified UMD build, including the upstream copyright banner. It is used by the graph view (`js/views/graph.js`).
- `rx.js` is the unmodified minified UMD build, including the upstream license banner. It is not loaded by any page today; it is retained for planned reactive UI work. Do not add a `<script>` tag for it until a view actually uses it.
- When adding a vendored file: record it here with the exact version and hash, keep the upstream license banner intact, and prefer the minified distribution build.
