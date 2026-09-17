# Diagrams — contracts and analytics

Scaffold diagrams. Nothing here is implemented.

## 1. Contract object flow (up / down)

```mermaid
flowchart TB
  subgraph L0["L0 — MCU / strap"]
    SAMPLES["samples + quality bits"]
  end

  subgraph PN["Personal node"]
    L1["L1 snapshot\nquality · short HRV · cadence\nshort entropy / complexity"]
    RSSI["RSSI table or sibling channels"]
    REL["relative status\n(last briefs + ages)"]
  end

  subgraph UN["Unit node — formation; or personal node on a body graph"]
    L2["L2 person-pattern\nRQA · entropy"]
    L3["L3 pairwise matrix\nphysio CRQA · movement CRQA\nrelative phase · proximity class"]
    L4["L4 regime\nnetwork summaries · MdRQA\ncluster-phase · components"]
    BRIEF["Brief (down)\nregime · self · neighbors · citations"]
  end

  SAMPLES -->|primary pair| L1
  L1 --> RSSI
  L1 -->|feature snapshot + quality| L2
  RSSI --> L3
  L2 --> L3
  L3 --> L4
  L4 --> BRIEF
  BRIEF -->|down| REL
  REL -.->|degraded picture| PN
```

**Text equivalent**

```text
MCU/strap (L0 samples + quality)
        │  primary pair (many channels per sid allowed)
        ▼
Personal node
  L1 snapshot (quality, short HRV, cadence, short entropy)
  RSSI table / sibling channels
  relative status (last briefs)
        │  features + quality   (raw on pull only)
        ▼
Unit node (formation) or the same personal node (body graph)
  L2 person-pattern (RQA, entropy)
  L3 pairwise matrix (physio, movement, relative phase, proximity)
  L4 regime (network + MdRQA + cluster-phase + components)
        │
        ▼
  Brief ──► back (relative status or self-cue)
```

## 2. Analytics by layer — univariate vs crossed / multi regularity

Every layer assesses **both** univariate regularity and crossed / multi
regularity. Higher layers may also request more detailed data.

```mermaid
flowchart LR
  subgraph Uni["Univariate regularity"]
    U0["L0 short complexity proxies"]
    U1["L1 short-window entropy / complexity"]
    U2["L2 RQA + entropy of person series"]
    U3["L3 (per series inside a pair)"]
    U4["L4 entropy of graph descriptors"]
  end

  subgraph Cross["Crossed / multi regularity"]
    C0["L0 local dual-channel cross if available"]
    C1["L1 simple local cross measures"]
    C2["L2 trends that later feed graph monitoring"]
    C3["L3 physio CRQA · movement CRQA\nrelative phase · cross-entropy style"]
    C4["L4 pairwise matrix network\nMdRQA / joint RQA\ncluster-phase ρ_group / ρ_k\ncomponent structure"]
  end

  U0 --> U1 --> U2 --> U3 --> U4
  C0 --> C1 --> C2 --> C3 --> C4
```

| Layer | Univariate regularity | Crossed / multi regularity |
|:------|:----------------------|:---------------------------|
| **L0** | Short complexity proxies | Local dual-channel cross if two sensors exist |
| **L1** | Short-window entropy / complexity, short HRV | Simple local cross; trends for a cue or upstream |
| **L2** | RQA + entropy of the longer series | Features the graph will use to monitor that node |
| **L3** | Per-series descriptors inside each pair | Physio CRQA, movement CRQA, relative phase, causal effort vs directional phase |
| **L4** | Entropy (and RQA) of the graph descriptors | Network summaries, MdRQA / joint recurrence, cluster-phase, component count → regime |

## 3. In-phase / out-of-phase path (L3 → L4)

```mermaid
flowchart TB
  PAIR["L3 pairwise edges\nphysio · movement · proximity"]
  PHASE["Relative phase / phase locking\neffort-phase vs directional-phase"]
  CRQA["CRQA matrix\nDET · diagonal entropy · etc."]
  NET["Network summaries\nmean coupling · components · modularity"]
  CP["Cluster-phase\nρ_group · ρ_k · φ to cluster"]
  MDRQA["MdRQA / joint RQA\non graph descriptors"]
  REGIME["Regime label\ntogether / intended split / unintended fragment"]

  PAIR --> PHASE
  PAIR --> CRQA
  CRQA --> NET
  PHASE --> CP
  NET --> REGIME
  CP --> REGIME
  MDRQA --> REGIME
```

```text
L3 pairwise edges
  ├─ relative phase / phase locking (effort vs directional)
  └─ CRQA matrix ──► network summaries (components, mean coupling)
         │
         ▼
L4  cluster-phase (ρ_group, ρ_k, relative phase to cluster)
    + MdRQA / joint RQA on graph descriptors
    + entropy of those descriptors
         │
         ▼
    regime: together | intended split | unintended fragment
```

## 4. Request path for more detailed data

```text
Higher layer (unit node / later archive)
        │  request (schema + quality required)
        ▼
Personal node / L0
        │  may return longer windows, higher-rate segments,
        │  extended diagnostics, extra cross measures
        ▼
Requester decides trust from quality + schema version
```

Default remains the narrow feature bus. The capability is architectural;
policy is out of scope.

## See also

- [analytics.md](analytics.md)
- [architecture.md](architecture.md)
- [contracts.md](contracts.md)
