# `index.md` mermaid template

`grove render` writes a `graph TD` block into `.grove/index.md`. Node classes follow the palette below; do not edit them by hand.

```mermaid
graph TD
  Aa["A-NN: title"]:::area
  Ga["G-NN: title"]:::goal
  Wf["W-NN: title"]:::feature
  Wr["W-NN: title"]:::ready:::critical
  Dd["D-NN: title"]:::decision
  Qq["Q-NN: title"]:::question
  Ws["W-NN: title"]:::spike
  Ba["B-NN: title"]:::assumption
  Tt["T-NN: title"]:::theme
  Yy["Y-NN: title"]:::da

  Wf ==>|blocks| Wr
  Ba -.->|targets| Wf
  Ba -- tests --> Qq
  Qq -->|asks| Ws
  Wf -->|implements| Dd
  Ws -->|produces| Yy
  Yy -->|distills| Dd
  Tt -->|causes| Wf

  classDef area fill:#5a1e4a,color:#fff
  classDef goal fill:#1e3a5f,color:#fff
  classDef theme fill:#2a4a3a,color:#fff
  classDef decision fill:#5a4a1e,color:#fff
  classDef question fill:#5a3a1e,color:#fff
  classDef assumption fill:#4a2d5a,color:#fff
  classDef spike fill:#3a3a5a,color:#fff
  classDef feature fill:#1e4a4a,color:#fff
  classDef da fill:#1f4e5f,color:#fff
  classDef ready fill:#2d5a27,color:#fff
  classDef progress fill:#3a4a6a,color:#fff,stroke:#fff,stroke-width:2px
  classDef done fill:#2d5a27,color:#fff,stroke:#fff,stroke-width:2px
  classDef rejected fill:#5a5a5a,color:#fff
  classDef blocked fill:#5a2d2d,color:#fff
  classDef critical stroke:#ff0,stroke-width:3px
```

The longest unfinished `blocks` chain is annotated `:::critical`. When multiple work items are `ready`, `grove next` picks from this set first.

**Edge link styles (automatic):** thick `==>|blocks|`; dotted `-.->|targets|`; plain arrow `-->|label|` for all other labels (`produces`, `causes`, `asks`, …).
