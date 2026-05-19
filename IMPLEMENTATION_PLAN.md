# Pseudo Implementation Plan

## 1. Product Goal

Build a local-first desktop app that helps a lawyer pseudonymize confidential text before using it elsewhere. The product should make it fast to paste a sensitive draft, detect identifying spans, review and adjust replacements, and copy a pseudonymized result.

v0 is LLM-first. The local model runs from the first analysis. Deterministic detectors handle structured types, email and URL, where regex is cheap and reliable; the LLM handles everything else. The product's value is the review loop around an imperfect model, not the model itself.

Privacy invariant: user content never leaves the machine. No source text, detected findings, replacement map, pseudonymized output, saved project content, imported document content, or manually marked text may ever be sent to any network service. Pseudonymization always runs locally, including when the optional model is installed. The app may use the network only for explicit non-content operations such as downloading a model, license activation, update checks, or enterprise/admin metadata sync, and those operations must never include user content or pseudonymization artifacts.

## 2. Core User Flow

### Flow

1. User opens the app to an empty editor.
2. User pastes confidential text.
3. The pasted or edited text enters `DIRTY_NEEDS_ANALYSIS`; analysis does not run automatically in v0.
4. User clicks `Analyze`.
5. If the model is missing, the app offers to download the GGUF file into the app-managed model directory.
6. Rust backend runs the email/URL deterministic pre-pass and local LLM analysis.
7. Backend returns findings, replacement groups, and an initial pseudonymized preview.
8. UI highlights findings in source text, shows replacement groups, and renders the pseudonymized preview.
9. User edits replacement labels, disables replacements, or manually marks missed spans.
10. Preview updates locally after replacement edits.
11. User clicks `Copy result`.
12. Backend recomputes the final result against the current source text hash and replacement groups.
13. UI copies the backend-confirmed pseudonymized text.

### Analysis State Model

v0 uses explicit analysis. Pasting or typing should not trigger analysis automatically.

States:

- `EMPTY`: editor has no meaningful text. `Analyze` and `Copy result` are disabled.
- `DIRTY_NEEDS_ANALYSIS`: source text has changed since the last analysis, or no analysis exists for the current text. `Analyze` is enabled; highlights, replacement editing, manual marking, and `Copy result` are disabled.
- `ANALYZING`: backend analysis is in progress. Inputs may remain visible, but `Analyze` and `Copy result` are disabled.
- `ANALYZED_READY`: findings, groups, highlights, and preview correspond to the current text hash. Replacement editing, manual marking, and `Copy result` are enabled when group state is valid.
- `ERROR`: last analysis failed. Keep the user's text visible and show an actionable error. Deterministic/manual fallback should remain available when model failure permits it.

Dirty-state UI:

- When text is edited after analysis, dim old highlights or clear them; do not leave them looking authoritative.
- Freeze or clear the old preview; do not silently update it against stale findings.
- Show concise state text such as `Review paused · analyze again`.
- Disable `Copy result` until the backend has analyzed the current text hash.

Backend sync rule:

- Every `AnalysisResult` includes `sourceTextHash`.
- `apply_replacements` and `recompute_analysis` must receive `expected_text_hash`.
- If the current text hash does not match, the backend rejects the request with a stale-analysis error.
- The frontend includes `request_id` on analysis/manual/recompute calls and ignores stale responses whose `request_id` no longer matches the latest request.

## 3. Initial Sensitive Information Types

Start with a practical taxonomy focused on removing identifying context before LLM use:

- `PERSON_NAME`: real names and initials
- `ORGANIZATION`: companies, institutions, departments
- `ROLE_OR_POSITION`: job titles, functions, seniority, team roles, or rare positions that may identify someone in context
- `LOCATION`: addresses, cities, countries, facilities
- `EMAIL`: email addresses
- `PHONE`: phone numbers
- `DATE`: dates tied to personal events or records
- `ID_NUMBER`: customer IDs, case numbers, account numbers, passport-like identifiers
- `URL`: links that may reveal identity or private systems
- `OTHER_SENSITIVE`: sensitive span that does not fit the above

Do not treat medical or financial facts as sensitive by themselves in the initial taxonomy. They should be pseudonymized only when they identify a person, organization, location, account, or other traceable entity. For example, "diabetes" can remain, while "Dr. Jane Doe at ACME Clinic" should be detected through name, role, and organization.

The UI should expose the type labels in human-friendly form, but the internal representation should use stable enum-style values.

In v0, only `EMAIL` and `URL` have deterministic detectors. All other types are LLM-only. Add more deterministic detectors only when there is evidence the LLM mis-detects a specific type often enough to matter.

## 4. Technical Stack

### Desktop Shell

- Tauri 2
- Rust backend commands for file-safe local operations and model orchestration
- Frontend built with React, TypeScript, and Vite

### Clean Core Boundary

Keep `pseudo-core` clean enough that it could be open-sourced later if buyers ask for auditability, but do not treat open-source as a v0 goal or differentiator. Presidio already exists as the broad open-source PII/anonymization framework; `pseudo-core` is only the app's small Rust range/replacement core.

- `pseudo-core` owns deterministic detectors, chunking, grouping, range-based replacement, overlap resolution, manual-finding validation, and shared data types.
- `pseudo-core` must be a pure Rust library with no Tauri dependency, no license checks, no model download code, no model runtime orchestration, no telemetry, no auto-update logic, and no network calls of any kind.
- The Tauri binary crate owns app commands, UI-facing orchestration, model runtime orchestration, packaging integration, and later commercial infrastructure.
- `pseudo-core` remains free of `llama-cpp-2`, model files, and inference code. The `llama-cpp-2` dependency lives only in the closed Tauri app crate or a dedicated closed `pseudo-runtime` crate inside the app workspace.
- Every dependency in `pseudo-core` must be permissively licensed: MIT, Apache-2.0, BSD, ISC, or MPL-2.0. Do not allow GPL or AGPL dependencies in the core.
- Add `cargo deny` in CI from day one to check dependency licenses and catch accidental policy drift while the codebase is still small.

Ship a small `pseudo-cli` wrapper around `pseudo-core`. A command such as `pseudo-cli analyze < input.txt > output.json` gives tests a UI-free path and keeps the core easy to inspect.

### Frontend

- Keep pure utilities in `src/lib/core`.
- Keep Tauri bindings and app glue in `src/lib/app`.
- Use shared TypeScript types that mirror Rust data structures.
- Do not put privacy-critical logic only in the frontend. Frontend preview may be optimistic, but backend replacement is canonical before copy/export.

### Local Model Runtime

The app uses `llama.cpp` (MIT) as the inference engine, via the `llama-cpp-2` Rust bindings (`utilityai/llama-cpp-rs`), to run Qwen3-1.7B in GGUF format (Apache-2.0). Inference runs in-process inside the Tauri app crate or a dedicated closed `pseudo-runtime` crate. There is no Python runtime, virtualenv, localhost HTTP server, or out-of-process model service.

v0 uses Q4_K_M only. Current target: `ggml-org/Qwen3-1.7B-GGUF:Q4_K_M`, unless the Qwen namespace publishes an official Q4_K_M file before implementation.

Backend strategy for v0:

- macOS: Metal backend, especially for Apple Silicon.
- Windows x86_64: CPU backend. Windows support is required in v0 because the app should be sendable as a small installer or archive.
- Linux CPU support can follow the same path, but it is not required for the first handoff.

Licensing notes to retain in product docs:

- Qwen3-1.7B and Qwen GGUF weights are Apache-2.0. If the selected Q4_K_M file comes from `ggml-org/Qwen3-1.7B-GGUF`, preserve that repo's upstream license metadata as well as the original Qwen license reference.
- `llama.cpp` is MIT. Include its license in future third-party notices.
- `llama-cpp-2` is MIT OR Apache-2.0. Include it in future third-party notices.

## 5. High-Level Architecture

```text
React UI
  |
  | Tauri command: analyze_text(request_id, text)
  v
Rust backend (app crate)
  |
  | runs deterministic detectors (pseudo-core)
  | calls in-process llama.cpp via llama-cpp-2
  | with offset-preserving chunk batches
  v
llama.cpp inference (in-process, no IPC)
  |
  | loads local Qwen3-1.7B GGUF from app-managed path
  | returns grammar-constrained JSON findings
  v
Rust backend
  |
  | validates and normalizes spans (pseudo-core)
  v
React UI
  |
  | highlights findings
  | groups recurring sensitive parts
  | applies replacements locally for preview
```

## 6. Data Model

### Finding

```ts
type SensitiveType =
  | "PERSON_NAME"
  | "ORGANIZATION"
  | "ROLE_OR_POSITION"
  | "LOCATION"
  | "EMAIL"
  | "PHONE"
  | "DATE"
  | "ID_NUMBER"
  | "URL"
  | "OTHER_SENSITIVE";

type FindingSource = "DETERMINISTIC" | "LLM" | "MANUAL";

type Finding = {
  id: string;
  type: SensitiveType;
  start: number;
  end: number;
  text: string;
  source: FindingSource;
  confidence?: number;
  needsReview?: boolean;
};
```

### Replacement Group

```ts
type ReplacementGroup = {
  id: string;
  type: SensitiveType;
  original: string;
  normalizedOriginal: string;
  replacement: string;
  findingIds: string[];
  enabled: boolean;
  aliases?: string[];
  aliasOfGroupId?: string;
};
```

### Analysis Result

```ts
type AnalysisResult = {
  requestId: string;
  sourceTextHash: string;
  findings: Finding[];
  groups: ReplacementGroup[];
  pseudonymizedText: string;
  warnings: string[];
};
```

### Preview Sync Strategy

Use pure-frontend optimistic preview for replacement edits:

- `analyze_text` returns backend-confirmed findings, groups, and initial preview.
- While text is unchanged, replacement text edits, enabled toggles, and manual findings update the preview synchronously in the frontend.
- Do not call the backend for every replacement edit.
- Manual marking calls the backend once to validate the selected range and return a canonical finding.
- Before copy, always call `apply_replacements`; copy only the backend-confirmed result.
- If frontend preview and backend result differ before copy, trust the backend and update the preview to match.

## 7. Analysis Pipeline

### Step 1: Deterministic Detection Pre-Pass

v0 deterministic detectors cover `EMAIL` and `URL` only, using simple regex with no locale handling. All other sensitive types are detected by the LLM.

Email:

- Accept ordinary address forms with a local part, `@`, domain labels, and a 2+ character TLD, using a compiled regex such as `(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b`.
- Reject matches embedded inside larger non-email tokens when obvious.

URL:

- Accept `http://` and `https://` URLs.
- Accept `www.` URLs.
- Do not detect arbitrary bare domains in v0 because false positives are high.

### Step 2: Offset-Preserving Chunking

Chunking must preserve exact character offsets into the original text. Final findings are ranges in the original source string, not in normalized text.

For v0, implement `chunkTextForAnalysis` with `{chunkIndex, start, end, text}` chunks:

1. Split on blank lines.
2. If a chunk is still too large, split near sentence-like punctuation.
3. If a chunk is still too large, split near whitespace before a maximum character budget.

All final findings must be validated against offsets in the original source text.

### Step 3: LLM Span Detection

Prompt the local Qwen model with small batches of chunks and constrain the response to schema-conformant JSON with a GBNF grammar.

Example prompt shape:

```text
You identify sensitive information in text for local pseudonymization.
Return only JSON. Do not rewrite the text.

Sensitive types:
PERSON_NAME, ORGANIZATION, ROLE_OR_POSITION, LOCATION, EMAIL, PHONE, DATE,
ID_NUMBER, URL, OTHER_SENSITIVE.

For each chunk, return exact substrings that appear in the chunk.
Focus on information that can identify or trace a person, organization, or private context.
Do not flag medical or financial facts unless the exact span itself is identifying.

Input:
[
  {"chunkIndex": 0, "text": "..."},
  {"chunkIndex": 1, "text": "..."}
]

Output schema:
{
  "findings": [
    {
      "chunkIndex": 0,
      "text": "exact substring",
      "type": "PERSON_NAME",
      "confidence": 0.0,
      "reason": "short reason"
    }
  ]
}
```

The LLM should not be responsible for character offsets. Treat returned text values as sensitive surface forms, then let deterministic backend code convert them into ranges:

- search for all exact occurrences of each returned surface form within the source chunk range
- discard surface forms that do not occur exactly in the source chunk
- resolve all overlaps through the canonical overlap resolver described below
- keep every non-overlapping matched range and group repeated occurrences later

For example, if a chunk contains `Jane told Jane Doe that Jane should call back.` and the LLM returns `Jane`, `Jane Doe`, and `Jane`, the backend should resolve ranges for all exact matches, remove the overlapping `Jane` inside `Jane Doe`, and group the two remaining `Jane` occurrences together.

### Step 4: Validation and Normalization

Every finding must pass validation:

- `start < end`
- `text[start..end]` equals the finding's `text`
- span does not cross invalid string boundaries
- type is one of the known `SensitiveType` values
- confidence is clamped to `0.0..1.0`
- warnings are attached rather than silently ignored

Normalize for grouping only:

- trim surrounding whitespace
- collapse internal whitespace to one space
- lowercase
- preserve the original surface form for display and replacement

### Step 5: Canonical Overlap Resolution

All findings, deterministic, LLM, and manual, must pass through one canonical overlap resolver before grouping.

Priority function:

1. Manual findings beat all automatic findings.
2. Deterministic findings beat LLM findings for the exact same span or overlapping structured span.
3. Higher confidence beats lower confidence when source priority is the same.
4. Longer span beats shorter span when one contains the other and priority is otherwise similar.
5. Earlier start offset beats later start offset as a deterministic tie-breaker.
6. Stable type order breaks remaining ties: `EMAIL`, `URL`, `ID_NUMBER`, `PHONE`, `DATE`, `PERSON_NAME`, `ORGANIZATION`, `ROLE_OR_POSITION`, `LOCATION`, `OTHER_SENSITIVE`.

Algorithm:

1. Build candidates with `start`, `end`, `length`, `sourcePriority`, `confidence`, `typeRank`, and original index.
2. Sort candidates by priority descending, then start ascending for deterministic tie-breaks.
3. Iterate sorted candidates.
4. Accept a candidate if it does not overlap any accepted candidate.
5. If it overlaps only lower-priority accepted candidates, replace those accepted candidates with the higher-priority candidate.
6. If neither side clearly dominates a partial overlap, keep the already accepted candidate and attach a warning for the discarded candidate.
7. Sort final accepted findings by `start` ascending.

Property tests:

- output ranges are sorted and non-overlapping
- every output range exactly matches the source slice
- resolution is deterministic regardless of input ordering
- adding a lower-priority overlapping candidate cannot remove a higher-priority winner
- contained spans obey the longer-span rule after source priority
- same-span type disagreement resolves through source priority, confidence, then type order

## 8. Replacement Strategy

### Grouping

After overlap resolution, group findings by:

- `type`
- normalized original text

This means every exact normalized repeat gets one replacement suggestion.

v0 co-reference scope:

- Exact-normalized grouping only.
- Do not automatically group `Jane`, `Jane Doe`, `Ms. Doe`, or pronouns together.
- Rely on manual marking for aliases in v0.
- Keep `aliases` and `aliasOfGroupId` in the data model for later reviewable alias grouping, but leave them unused by default in v0.

Do not implement substring-containment grouping in v0. It creates tempting false positives and requires parent/child review behavior the first build does not need.

### Suggested Replacements

Use deterministic numbering by type and first occurrence:

| Type | Replacement Pattern |
| --- | --- |
| `PERSON_NAME` | `[PERSON_1]` |
| `ORGANIZATION` | `[ORG_1]` |
| `ROLE_OR_POSITION` | `[ROLE_1]` |
| `LOCATION` | `[LOCATION_1]` |
| `EMAIL` | `[EMAIL_1]` |
| `PHONE` | `[PHONE_1]` |
| `DATE` | `[DATE_1]` |
| `ID_NUMBER` | `[ID_1]` |
| `URL` | `[URL_1]` |
| `OTHER_SENSITIVE` | `[REDACTED_1]` |

Number by first occurrence offset in the current analysis result. Re-analysis regenerates suggestions from scratch in v0.

### Readiness Summary

The readiness summary is computed from analysis state, findings, replacement groups, and warnings. Use one deterministic function for the status text and counts.

Definitions:

- A finding `needsReview` when its type is `OTHER_SENSITIVE`, its confidence is below `0.70`, or it has a warning from validation/model parsing.
- A replacement group `needsReview` when any occurrence needs review, when the group replacement is empty/whitespace, or when an enabled group has an invalid replacement token.
- A manual finding does not need review merely because it is manual; it needs review only if its replacement group has no chosen replacement or has an invalid replacement.
- A disabled group is user intent, not `needsReview`; count it separately as `disabled`.

Status rules:

- `EMPTY`: show `Paste text to begin`; `Copy result` disabled.
- `DIRTY_NEEDS_ANALYSIS`: show `Review paused · analyze again`; `Copy result` disabled.
- `ANALYZING`: show `Analyzing...`; `Copy result` disabled.
- `ERROR`: show a concise actionable error; `Copy result` disabled unless there is a clearly valid previous backend-confirmed result for unchanged text.
- `ANALYZED_READY` with zero findings: show `No findings`; `Copy result` enabled because copying unchanged text may still be useful.
- `ANALYZED_READY` with findings and `needsReviewCount = 0`: show `Ready to copy · {findingCount} findings · {enabledCount} replacements enabled`.
- `ANALYZED_READY` with `needsReviewCount > 0`: show `Manual review recommended · {needsReviewCount} need review · {enabledCount} replacements enabled`.
- Include disabled counts only as secondary details, e.g. `{disabledCount} disabled`, not as blockers.

Thresholds:

- Deterministic findings should generally use confidence `1.0`.
- LLM findings below `0.70` need review.
- LLM findings without confidence should default to `0.50` and need review.
- `OTHER_SENSITIVE` always needs review regardless of confidence.

### Applying Replacements

Apply replacements automatically in the preview after every completed analysis and after every user edit to the replacement map. The canonical implementation should live in Rust and apply by sorted character ranges, from end to start, not by naive global string replacement. The frontend mirrors that algorithm for optimistic preview only while in `ANALYZED_READY`. This prevents accidental changes to text outside confirmed spans and preserves offsets during replacement.

Disabled groups are skipped.

### Manual Marking

The user must be able to select a span in the source text and mark it as sensitive. Manual findings should be validated like any other finding, then get highest priority in overlap resolution.

## 9. Frontend Layout

### Main Layout

Use a practical working layout:

- top toolbar with `Analyze`, `Clear`, and `Copy result`
- left pane: source editor with highlights
- right pane: replacement groups
- lower or adjacent pane: pseudonymized preview
- compact readiness summary near `Copy result`

Top toolbar:

- `Analyze`: enabled when state is `DIRTY_NEEDS_ANALYSIS`.
- `Clear`: clears source text, findings, groups, preview, and in-memory state.
- `Copy result`: enabled only when the backend-confirmed result matches the current text hash and group state is valid.

Missing model state:

- If the model is missing, show a compact local-model panel with model name, approximate download size, destination folder, and a `Download model` action.
- The app must remain small enough to send directly; the model is downloaded after install.
- Model download UI must never ask for an account and must not mention pricing, trials, or activation.

Left pane states:

- Empty input: plain placeholder such as `Paste text here`.
- Dirty analyzed text: dim or clear stale highlights and show `Review paused · analyze again`.
- Analyzing: keep text visible and show progress state.
- Error: keep text visible and show concise backend error.

### Highlight Behavior

- Highlight findings in the source editor without changing text.
- Clicking a highlight should focus the matching replacement group.
- Hovering or focusing a replacement group should indicate corresponding highlights.
- Disabled groups should remain visible but visually muted.
- Manual selection should not corrupt offsets.

## 10. Tauri Commands

Initial command surface:

```rust
#[tauri::command]
async fn analyze_text(request_id: String, text: String) -> Result<AnalysisResult, AppError>;

#[tauri::command]
async fn apply_replacements(text: String, expected_text_hash: String, groups: Vec<ReplacementGroup>) -> Result<String, AppError>;

#[tauri::command]
async fn create_manual_finding(request_id: String, text: String, start: usize, end: usize, type_: SensitiveType) -> Result<Finding, AppError>;

#[tauri::command]
async fn recompute_analysis(request_id: String, text: String, expected_text_hash: String, findings: Vec<Finding>, groups: Vec<ReplacementGroup>) -> Result<AnalysisResult, AppError>;

#[tauri::command]
async fn load_model() -> Result<ModelStatus, AppError>;

#[tauri::command]
async fn unload_model() -> Result<(), AppError>;

#[tauri::command]
async fn get_model_status() -> Result<ModelStatus, AppError>;

#[tauri::command]
async fn get_model_download_status() -> Result<ModelDownloadStatus, AppError>;

#[tauri::command]
async fn start_model_download() -> Result<ModelDownloadStatus, AppError>;

#[tauri::command]
async fn cancel_model_download() -> Result<ModelDownloadStatus, AppError>;
```

The frontend should call `analyze_text`, receive findings and replacement groups, show the backend-generated initial pseudonymized preview, and keep an optimistic preview in sync with replacement edits while in `ANALYZED_READY`. Manual marking should call `create_manual_finding`, append the returned finding, then call `recompute_analysis` with the explicit current findings, groups, and expected text hash. `Copy result` should call `apply_replacements` first and copy the backend-confirmed result. Backend commands that depend on a prior analysis must reject mismatched `expected_text_hash`. `request_id` is echoed back by the frontend command wrapper or response envelope so stale responses can be ignored.

`ModelStatus` should expose `{ loaded: boolean, modelPath?: string, quantization?: string, backend: "metal" | "cpu", loadMs?: number, residentMemoryMb?: number }`. Model lifecycle is in-process and owned by the Rust app/runtime layer.

`ModelDownloadStatus` should expose `{ state: "not_started" | "downloading" | "complete" | "error" | "cancelled", modelName: string, destinationPath: string, bytesDownloaded: number, totalBytes?: number, error?: string }`. It must never include user text or analysis metadata.

## 11. Local Model Service (In-Process)

Inference runs inside the Tauri app crate, or a dedicated closed `pseudo-runtime` crate, via `llama-cpp-2`. There is no separate process, no IPC, no localhost port, and no Python runtime.

Responsibilities:

- Locate the local GGUF file.
- Download the v0 GGUF file into the app-managed model directory when missing and explicitly requested by the user.
- Load the model on first analysis, not at app launch, so cold start stays fast.
- Unload on app quit. Idle-unload tuning is out of scope for v0.
- Run inference with a JSON-grammar-constrained sampler using llama.cpp GBNF grammars so responses stay schema-conformant.
- Run Qwen3 span extraction in non-thinking mode, using `/no_think` or the equivalent runtime configuration, because chain-of-thought tokens add latency without improving a structured extraction result.
- Never log user text, chunks, prompts, finding surface forms, replacement maps, or pseudonymized output.
- Stay outside `pseudo-core`; inference, GGUF loading, model lifecycle, and runtime orchestration are closed app/runtime concerns.

The grammar-constrained model response should match this schema:

```json
{
  "findings": [
    {
      "chunkIndex": 0,
      "text": "Jane Doe",
      "type": "PERSON_NAME",
      "confidence": 0.82,
      "reason": "person name"
    }
  ],
  "warnings": []
}
```

`model_runtime` passes `(chunk_index, surface_form, type, confidence)` tuples to `pseudo-core` for offset resolution, validation, overlap handling, grouping, and replacement. The model should never be trusted with source offsets directly.

### Model Discovery

The runtime should resolve the model path in this order:

1. Explicit app config path.
2. `PSEUDO_MODEL_PATH`.
3. App-managed model directory.

### Model Download Manager

v0 includes a minimal model download manager so the app can be sent as a small binary without bundling the GGUF file.

- Download the selected Q4_K_M GGUF file from Hugging Face into the app-managed model directory.
- Start only after explicit user action from the missing-model state.
- Show model name, approximate size, destination folder, progress, and errors.
- Support cancellation. Resume is nice to have, but not required for the first handoff.
- Validate the completed file with a pinned size and checksum or manifest before loading it.
- Store upstream license metadata alongside the model file when practical.
- Never send user content, findings, prompts, replacement maps, or pseudonymized output during model download.

### Memory, Lifecycle, and Concurrency

- Budget roughly 1.5-2 GB resident memory for Q4_K_M Qwen3-1.7B with default KV-cache settings, then tune after measurement on reference hardware.
- Keep the model loaded while analysis is active.
- If load or inference fails, fall back to deterministic/manual mode and show `Local model unavailable. Basic detection is still available.`
- Do not include user text, findings, prompts, or replacement maps in crash logs or crash UI. Crash reports, if added later, must be metadata-only and opt-in.
- Serialize analyses through a single `LlamaSession` behind a mutex for v0.
- Parallel chunk batching within one analysis is allowed; concurrent LLM analyses are not worth the complexity in v0.

## 12. Future Commercial Considerations

Pricing, licensing, trials, and tier-gating are out of scope for v0. The current goal is a working tool used repeatedly by a single trusted user. Commercial structure will be designed after that user has used the tool on real work for several weeks and additional buyers have been validated.

## 13. Privacy and Security Requirements

- All analysis must run locally.
- No analytics or telemetry in v0.
- Do not persist pasted text unless the user explicitly saves a project later.
- Do not log user text in Rust or the frontend console.
- Never send user content or pseudonymization artifacts to any network service. This includes pasted text, imported document content, saved project content, manually marked text, findings, replacement maps, and pseudonymized output.
- Model download is the only v0 network feature. It must live outside `pseudo-core` and send no user content or pseudonymization artifacts.
- Non-content network features added later, such as update checks or commercial activation, must live outside `pseudo-core`.
- `pseudo-core` must make no network calls of any kind. It must not include update checks, license checks, model downloads, crash reporting, telemetry, HTTP clients, socket clients, or model runtime orchestration.
- All network activity must live in the outer app/runtime crates and be auditable at that boundary.
- The local model runs in-process. No localhost ports are opened.
- No external network connections are made during analysis.
- Clear in-memory state when the user clicks `Clear`.
- Document that the app assists pseudonymization but does not guarantee formal anonymization for every regulatory or professional standard.

## 14. Project Structure

Target structure:

```text
pseudo/
  package.json
  vite.config.ts
  src/
    main.tsx
    App.tsx
    components/
      EditorPane.tsx
      HighlightLayer.tsx
      ReplacementPanel.tsx
      PreviewPane.tsx
      Toolbar.tsx
    lib/
      core/
        types.ts
        replacements.ts
        preview.ts
        state.ts
      app/
        tauriApi.ts
        modelStatus.ts
        modelDownloadStatus.ts
  src-tauri/
    Cargo.toml
    tauri.conf.json
    pseudo-core/
      Cargo.toml
      src/
        lib.rs
        types.rs
        rules.rs
        chunking.rs
        grouping.rs
        replacement.rs
        manual.rs
        overlap.rs
    pseudo-cli/
      Cargo.toml
      src/
        main.rs
    app/
      Cargo.toml
      src/
        main.rs
        commands.rs
        analysis.rs
        model_download.rs
        model_runtime.rs
  deny.toml
  docs/
    implementation-plan.md
```

This repository currently contains the plan at the root. Once the app scaffold exists, move or copy this document to `docs/implementation-plan.md`.

`pseudo-core` should be independently buildable and testable. The Tauri app crate and `pseudo-cli` should depend on it as consumers rather than duplicating deterministic logic. The CLI should be usable in CI for end-to-end deterministic analysis tests without launching the desktop UI.

`src-tauri/app/src/model_runtime.rs` owns the `llama-cpp-2` integration, model lifecycle, GBNF grammar definition, and inference loop. `src-tauri/app/src/model_download.rs` owns the minimal Hugging Face download flow, checksum validation, and app-managed model directory. If runtime code grows beyond roughly 1000 LOC, move it into a dedicated closed `src-tauri/pseudo-runtime` crate so `app/` stays focused on commands and orchestration. Do not move inference or download code into `pseudo-core`.

## 15. Testing Plan

### Unit Tests

Frontend:

- analysis state transitions: `EMPTY`, `DIRTY_NEEDS_ANALYSIS`, `ANALYZING`, `ANALYZED_READY`, and `ERROR`
- editing source text after analysis dims stale highlights, freezes stale preview, disables replacement editing/manual marking/copy, and shows `Review paused · analyze again`
- text chunking preserves offsets
- grouping repeated findings
- suggested replacement numbering by first occurrence
- exact-normalized grouping does not merge aliases such as `Jane`, `Jane Doe`, and `Ms. Doe`
- optimistic pseudonymized preview generation after analysis
- replacement edits and toggles update optimistic preview synchronously without backend calls
- final copy reconciles optimistic preview with backend-confirmed output
- stale analysis/manual/recompute responses are ignored by `requestId`
- disabled replacements are skipped
- readiness summary rules for `Ready to copy`, `Manual review recommended`, disabled groups, empty replacements, low-confidence findings, `OTHER_SENSITIVE`, and dirty/error states
- selection-to-manual-finding UI behavior
- `src/lib/core` utilities remain pure and do not import Tauri/app glue

Rust:

- `pseudo-core` email and URL deterministic detection
- `pseudo-core` has no Tauri, networking, license, model-download, model-runtime, telemetry, or update dependencies
- `pseudo-core` dependency licenses pass `cargo deny`
- model output validation: text-in-source check, confidence clamp, and unknown-type rejection
- exact surface-form matching from LLM output to source ranges
- overlap resolver property tests: output is sorted, non-overlapping, every winner exactly matches its source slice, resolver is deterministic under input permutation, and adding a lower-priority overlapping candidate cannot remove a higher-priority winner
- overlap resolver fixtures for deterministic-vs-deterministic conflicts, email-vs-URL/domain overlap, LLM partial overlap, same-span type disagreement, manual override, and longer-contained-span wins
- canonical grouping and replacement application from end to start
- manual finding validation for selected ranges
- stale text hash rejection for apply/recompute/copy paths
- model runtime status handling for loaded/unloaded/backend/quantization/load time/resident memory
- model download status handling, cancellation, checksum/manifest validation, and no-content request construction
- `app::model_runtime` or `pseudo-runtime` loads a fixture GGUF model and returns valid JSON for a known chunk
- GBNF grammar produces only schema-conformant JSON across a representative test set
- first LLM-assisted analysis cold-load completes under a documented reference budget
- model load failure and inference failure fall back to deterministic/manual analysis without clearing user work
- `pseudo-cli analyze < input.txt > output.json` returns deterministic findings and replacement groups without launching the UI

### Integration Tests

- paste text, run LLM-first analysis, show highlights, replacement groups, readiness summary, and pseudonymized preview
- edit analyzed source text and verify stale highlights are dimmed, copy is disabled, and `Analyze` is primary
- mark selected text as sensitive
- edit a replacement and see preview update immediately
- disable a replacement and see preview restore the original span
- copy backend-confirmed final result
- model load failure falls back to deterministic/manual behavior without losing the user's source text
- missing model state can download the model, validate it, store it in the app-managed directory, and then run analysis without app restart
- Windows CPU build opens, downloads the model, and runs one LLM-assisted analysis on a reference Windows machine

## 16. Build Plan

### v0: LLM-First Usable Build

- Scaffold Tauri 2 + React + TypeScript + Vite.
- Create the Rust workspace: `pseudo-core`, `pseudo-cli`, and app crate.
- Add `cargo deny` with permissive-license policy for `pseudo-core`.
- Implement `pseudo-core` types, grouping, range-based replacement, overlap resolution, and manual finding validation.
- Implement minimal deterministic detectors, email and URL only, in `pseudo-core`.
- Implement basic offset-preserving chunking in `pseudo-core`.
- Add `llama-cpp-2` integration in `model_runtime.rs` from the start.
- Add the minimal model downloader in `model_download.rs` from the start so the app can be sent without a bundled GGUF file.
- Implement GBNF grammar constraining LLM output to the `{findings: [...]}` schema.
- Implement the Tauri commands listed in Section 10.
- Implement the main layout: source editor with highlights, replacement panel, pseudonymized preview, and top toolbar with `Analyze`, `Clear`, and `Copy`.
- Implement the missing-model/download UI.
- Implement manual `Mark sensitive` selection-to-finding flow.
- Implement the five-state analysis state machine from Section 2.
- Implement deterministic readiness summary.
- Wire frontend optimistic preview with backend reconciliation on copy.
- Implement copy-to-clipboard.
- Add tests listed in Section 15.
- Build and smoke-test macOS Apple Silicon and Windows x86_64 CPU artifacts.

Deliverable: a small LLM-first local pseudonymization app that can be sent to the test user, download its own model, and run on Windows.

### Future Work

- Signed installers for broader distribution.
- License activation, trial mechanics, and paid tier UI.
- Bilingual detectors, including German month names, Swiss/German phone formats, and contextual ID labels.
- Phone, date, and ID deterministic detectors.
- Replacement memory across re-analysis within a session.
- First-run sample experience for unknown users.
- Type filters, confidence badges, ignore action, and reset-to-suggested.
- Settings screen, idle-unload tuning, and backend selection UI.
- Audit logging for Firm/Enterprise.
- Possible open-source release of `pseudo-core` and `pseudo-cli` if buyer auditability becomes important.
- Firm and Enterprise features, including admin console, SSO, and on-prem license server.

## 17. Key Risks and Mitigations

### LLM Output Is Not Reliably Structured

Mitigation:

- Keep deterministic email and URL detectors.
- Use llama.cpp GBNF grammar to constrain output to the expected JSON schema.
- Validate all model output.
- Discard invalid spans.
- Show warnings rather than failing the whole analysis.

### Pseudonymization May Miss Sensitive Data

Mitigation:

- Present the app as assisted review.
- Add manual marking in v0.
- Prefer conservative highlighting.
- Provide visible warning when model is unavailable.

### Packaging the Local Model Runtime

Mitigation:

- Use `llama-cpp-2`, Rust bindings to llama.cpp, for in-process inference. No Python runtime is bundled.
- Keep the app small by downloading the model after install instead of bundling the GGUF file.
- Keep the v0 downloader minimal: explicit user action, progress, cancellation, checksum/manifest validation, and clear errors.
- Smoke-test the Windows x86_64 CPU build on a real Windows machine before sending it.
- Keep model runtime behind a small `model_runtime.rs` boundary so later packaging work is isolated.
- Preserve license metadata for the selected GGUF file and runtime dependencies.

### Core Boundary Erodes

Mitigation:

- Keep deterministic logic in `pseudo-core` from the first implementation pass.
- Keep networking, commercial infrastructure, model runtime orchestration, telemetry, updates, and packaging out of `pseudo-core`.
- Enforce permissive dependency licenses with `cargo deny`.
- Use `pseudo-cli` and `pseudo-core` tests as the compatibility contract.
- Avoid real client fixtures and proprietary app details in core tests.
- Treat future open-sourcing as an optional trust/audit move, not as a product strategy.

### Replacing Text Naively Can Corrupt Output

Mitigation:

- Apply only validated character ranges.
- Sort ranges descending.
- Use Rust as the canonical replacement engine.
- Confirm the final copied result through the backend.
- Keep tests around overlapping and repeated spans.

## 18. Immediate Next Steps

1. Scaffold Tauri 2 + React + TypeScript + Vite project.
2. Create Rust workspace: `pseudo-core` library, `pseudo-cli` binary, and `app` Tauri binary.
3. Add `cargo deny` with permissive-license policy enforced for `pseudo-core`.
4. Implement `pseudo-core` types: `Finding`, `ReplacementGroup`, and `AnalysisResult`.
5. Implement `pseudo-core` overlap resolver with property tests.
6. Implement `pseudo-core` grouping and range-based replacement application.
7. Implement minimal email and URL deterministic detectors.
8. Implement basic offset-preserving chunking.
9. Implement manual finding validation.
10. Add `llama-cpp-2` to the app crate with a `model_runtime` module.
11. Add `model_download.rs` with Hugging Face download, progress status, cancellation, app-managed model directory, and checksum/manifest validation.
12. Implement GBNF grammar for the `{findings: [...]}` schema and unit-test it against a fixture model.
13. Implement the Tauri commands from Section 10.
14. Build the main UI: editor with highlights, replacement panel, pseudonymized preview, toolbar, and missing-model download panel.
15. Implement manual `Mark sensitive` flow.
16. Wire optimistic preview plus backend-reconciled copy.
17. Implement deterministic readiness summary.
18. Add unit tests for state transitions, grouping, replacement memory absence, replacement application, manual findings, stale-hash rejection, and model download status.
19. Build macOS and Windows x86_64 CPU artifacts and smoke-test Windows download plus one analysis.
20. Send the small app build to the test user.
