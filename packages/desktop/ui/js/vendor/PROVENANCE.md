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
| `three.module.min.js` | [three](https://www.npmjs.com/package/three) (MIT license) | 0.185.1 | `https://cdn.jsdelivr.net/npm/three@0.185.1/build/three.module.min.js` (identical bytes at `https://unpkg.com/three@0.185.1/build/three.module.min.js`) | `86bcee248b64f44bcfc23c331ae74619061957d59cab040171dcb6fb5900beb6` |
| `three.core.min.js` | [three](https://www.npmjs.com/package/three) (MIT license) | 0.185.1 | `https://cdn.jsdelivr.net/npm/three@0.185.1/build/three.core.min.js` (identical bytes at `https://unpkg.com/three@0.185.1/build/three.core.min.js`) | `05b2609338c76cd65daf74f3ac515bc9a5045e1b3b33edc07d8c9bd55250fa90` |

Notes:

- `d3.js` is the unmodified minified UMD build, including the upstream copyright banner. It is used by the graph view (`js/views/graph.js`).
- `rx.js` is the unmodified minified UMD build, including the upstream license banner. It is not loaded by any page today; it is retained for planned reactive UI work. Do not add a `<script>` tag for it until a view actually uses it.
- `three.module.min.js` and `three.core.min.js` are the unmodified minified ESM builds of three 0.185.1, including the upstream MIT banner. `three.module.min.js` imports `./three.core.min.js` relatively, so the pair must stay vendored side by side. They are loaded only through a dynamic import from the 3D graph view (`js/views/graph-3d.js`); do not add a `<script>` tag for them.
- When adding a vendored file: record it here with the exact version and hash, keep the upstream license banner intact, and prefer the minified distribution build.
