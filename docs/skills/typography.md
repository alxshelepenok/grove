# 7. Typography

Applies to this skill and to anything the agent writes into `.grove/glossary.md` or evidence/prose fields of `state.lock`.

**Definitions:**
- **Prose bullet/numbered item:** Full sentences in markdown lists (`- …` or `1. …`) that end with punctuation.
- **Phrase:** Short noun or gerund phrases used as titles, labels, or in structured data (e.g. glossary entries, state lock fields).

1. **Full stop.** Each prose bullet (`- …`) and each prose numbered item (`1. …`) ends with `.`. Checklist lines (`- [ ] …`) end with `.` as well. Exceptions: headings; YAML / Mermaid / code fences; table delimiter rows; identifiers and symbols inside formal blocks.

2. **Em dash (long dash, U+2014).** Do not use it. Do not imitate it with `--` or `---` except as required by Markdown table separator rows: those `---` cells are syntax, not punctuation. Prefer commas, colons, semicolons, parentheses, or a separate sentence.

3. **Hyphen and en dash.** Use ASCII hyphen `-` (U+002D) for compound words (`dual-track`) and inside code. Use en dash `–` (U+2013) only as the empty placeholder in table cells. Do not use en dash for aside punctuation; prefer alternative punctuation marks or rephrase sentences to avoid hyphens entirely.

4. **Arrows vs prose.** Keep ASCII `->`, `=>`, `-->` only as syntax (functions, implications in code, Mermaid edges). In prose, use words (`then`, `to`, `maps to`).

5. **Markdown tables.** Separator row immediately below the header, each column exactly `---` between pipes.

6. **English only.** All text content is written in English regardless of the surrounding conversation language.

7. **Sentence case.** Capitalise only the first word and proper nouns. Do not capitalise every word. Titles and phrases always start with a capital letter.

8. **No phase or stage prefixes.** Do not prepend labels such as `Phase 0:`, `Step 1:`, `Stage A:`, or similar. If ordering matters, encode it in the dependency graph via `blocks` edges, not in the text.

9. **Parentheses for scope qualifiers.** When a phrase must name the things it covers, append a parenthesised comma-separated list after the main phrase. Example: `Specification freeze (distribution.md schema, verification policy, worker JWT scope, .sequence)`. Do not inline that list with dashes or colons.

10. **Short main phrase.** The part before the parenthesis should be a noun phrase or gerund phrase of at most eight words. If you need more words, the item is probably two items.

11. **Field values that are not titles.** Short identifier-like fields (IDs, status values, cynefin tags, edge labels) follow their own grammar defined in `lockfile.md` and are exempt from the rules above.
