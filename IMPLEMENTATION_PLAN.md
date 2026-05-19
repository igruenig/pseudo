# Pseudonymization App Implementation Plan

## 1. Product Goal

Build a cross-platform desktop app with Tauri that helps users pseudonymize pasted text locally before sending it to an LLM. The app should analyze text in offset-preserving chunks, identify directly or indirectly identifying spans, group recurring occurrences, suggest generic replacements, and immediately show a pseudonymized result that the user can review before copying.

The first version should prioritize local privacy, transparent review, and predictable replacement behavior. The app can automatically apply suggested replacements after analysis, but the user must always be able to inspect and adjust the replacement map.

## 2. Core User Flow

1. User pastes text into the editor.
2. User clicks `Analyze`.
3. App splits the text into offset-preserving chunks and sends them to a local detection pipeline.
4. Sensitive spans are highlighted inline.
5. A side panel lists unique detected entities grouped by canonical text and type.
6. Each list item has:
   - detected value
   - sensitive info type
   - occurrence count
   - suggested pseudonym
   - editable replacement field
   - enable/disable toggle
7. App immediately applies enabled suggested replacements and shows the pseudonymized result.
8. User reviews and edits replacements.
9. The pseudonymized preview updates immediately after each replacement edit or toggle.
10. User copies the final text.

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

## 4. Technical Stack

### Desktop Shell

- Tauri 2
- Rust backend commands for file-safe local operations and model orchestration
- Frontend built with React, TypeScript, and Vite

### Frontend

- React + TypeScript
- State management with local React state first; introduce Zustand only if state becomes awkward
- Text editor:
  - MVP: controlled textarea plus overlay-based highlighting
  - Later: CodeMirror 6 or TipTap if richer text selection/review is needed
- Styling:
  - CSS modules or plain scoped CSS
  - Avoid heavy UI frameworks for the first version

### Local Model Runtime

The first commercial demo should ship as a small desktop app and download the local model after install. The plan should support three runtime/setup paths:

1. Primary MVP path: app-managed local model download plus sidecar Python service using `transformers`
   - Keeps the installer small
   - Lets the first-run UI explain that analysis remains local after setup
   - Supports progress, pause/resume, checksum validation, and clear setup errors
2. Developer/manual path: sidecar Python service using `transformers` with an existing local model path
   - Easiest path to Hugging Face model loading
   - Can use local cache without network
   - Keeps Rust/Tauri integration simple through localhost HTTP or stdio
3. Later native path: Rust-side inference via `candle`, `llama.cpp`, or ONNX
   - Better packaging story
   - Lower operational complexity after model format is settled

For the first build, use the Python sidecar because it reduces risk and gives faster iteration on prompts and parsing. Before public/commercial distribution, verify the model license allows the intended packaging, download flow, and commercial use.

## 5. High-Level Architecture

```text
React UI
  |
  | Tauri command: analyze_text(text)
  v
Rust backend
  |
  | starts/checks local model sidecar
  | sends offset-preserving chunk batches
  v
Python local inference service
  |
  | loads local Qwen model from Hugging Face cache
  | returns structured findings
  v
Rust backend
  |
  | validates and normalizes spans
  v
React UI
  |
  | highlights findings
  | groups recurring sensitive parts
  | immediately applies replacements locally
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

type Finding = {
  id: string;
  type: SensitiveType;
  text: string;
  start: number;
  end: number;
  chunkIndex?: number;
  confidence: number;
  reason?: string;
};
```

### Replacement Group

```ts
type ReplacementGroup = {
  id: string;
  type: SensitiveType;
  original: string;
  normalizedOriginal: string;
  occurrences: Finding[];
  suggestedReplacement: string;
  replacement: string;
  enabled: boolean;
};
```

### Analysis Result

```ts
type AnalysisResult = {
  textHash: string;
  findings: Finding[];
  groups: ReplacementGroup[];
  pseudonymizedText: string;
  warnings: string[];
};
```

`pseudonymizedText` is a derived value produced from the current text and enabled replacement groups. The Rust backend should be the canonical owner of validation, grouping, overlap resolution, and range-based replacement. The frontend may recompute an optimistic live preview for responsiveness, but final copy/export actions should use backend-confirmed output.

## 7. Detection Strategy

Use a hybrid pipeline instead of relying only on the LLM.

### Step 1: Deterministic Pre-Detection

Before invoking the LLM, run local regex/rule-based detection for:

- emails
- phone numbers
- URLs
- common date formats
- obvious ID-like tokens

Role and position detection should initially be handled by the LLM because job titles and functions are context-dependent and can be ambiguous.

Benefits:

- Faster
- More reliable for structured values
- Reduces model workload
- Provides baseline functionality if the LLM is unavailable

### Step 2: Offset-Preserving Chunking

Split text into deterministic chunks for LLM accuracy and latency, but do not make sentence boundaries part of the correctness model:

- preserve exact character offsets into the original text
- avoid changing whitespace
- prefer natural boundaries such as blank lines, paragraph breaks, newlines, and sentence-like punctuation
- support abbreviations reasonably well when choosing sentence-like boundaries
- fall back to bounded character windows when a paragraph is too large

For MVP, implement `chunkTextForAnalysis` with `{chunkIndex, start, end, text}` chunks. Split on blank lines first, then sentence-like boundaries, then whitespace near a maximum chunk size. The chunker may improve LLM input quality, but all final findings must still be validated against offsets in the original source text.

### Step 3: LLM Span Detection

Prompt the local Qwen model with small batches of chunks and request strict JSON output.

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
- prefer longer spans over shorter overlapping spans, so `Jane Doe` wins over `Jane` inside the same character range
- prefer deterministic findings over LLM findings when spans overlap
- keep every non-overlapping matched range and group repeated occurrences later

For example, if a chunk contains `Jane told Jane Doe that Jane should call back.` and the LLM returns `Jane`, `Jane Doe`, and `Jane`, the backend should resolve ranges for all exact matches, remove the overlapping `Jane` inside `Jane Doe`, and group the two remaining `Jane` occurrences together.

### Step 4: Validation and Normalization

The Rust backend should validate model output before passing it to the UI:

- discard findings whose text does not exist in the source chunk
- clamp confidence to `0..1`
- reject unknown types
- resolve overlapping spans deterministically
- prefer deterministic findings over LLM findings when spans overlap
- preserve original casing and whitespace

## 8. Replacement Strategy

Replacement should be deterministic and reviewable.

### Grouping

Group findings by:

- type
- normalized original text

Normalization rules:

- trim surrounding punctuation and whitespace
- collapse repeated whitespace
- lowercase for grouping
- keep original display text from first occurrence

### Suggested Replacements

Initial generic replacements:

| Type | Suggested Pattern |
| --- | --- |
| `PERSON_NAME` | `[PERSON_1]`, `[PERSON_2]` |
| `ORGANIZATION` | `[ORG_1]` |
| `ROLE_OR_POSITION` | `[ROLE_1]` |
| `LOCATION` | `[LOCATION_1]` |
| `EMAIL` | `[EMAIL_1]` |
| `PHONE` | `[PHONE_1]` |
| `DATE` | `[DATE_1]` |
| `ID_NUMBER` | `[ID_1]` |
| `URL` | `[URL_1]` |
| `OTHER_SENSITIVE` | `[SENSITIVE_INFO_1]` |

The numbering should be stable per analysis result and per type.

### Applying Replacements

Apply replacements automatically after every completed analysis and after every user edit to the replacement map. The canonical implementation should live in Rust and apply by sorted character ranges, from end to start, not by naive global string replacement. This prevents accidental changes to text outside confirmed spans and preserves offsets during replacement.

The frontend may run the same deterministic algorithm for immediate preview updates, but the backend remains the source of truth. Before copying or exporting, ask the backend to recompute the pseudonymized result from the current source text and replacement groups.

Provide an optional later feature: "replace all exact matches" for user-approved recurring text missed by the model.

### Manual Marking

Manual user markings are first-class findings. In the MVP, the user should be able to select text in the editor, choose `Mark sensitive`, pick a sensitive type, and create a validated finding from the exact selected range. The manual finding should flow through the same grouping, replacement, highlighting, and range-based replacement logic as deterministic and LLM findings.

## 9. Frontend Layout

### Main Screen

Use a two-pane desktop layout:

- Left pane: source text editor with highlighted identifying spans
- Right pane: replacement review list

Under or beside the source editor, depending on available window width, show the pseudonymized result preview. The result preview should be populated immediately after analysis and update live as the replacement list changes.

Top toolbar:

- Analyze
- Clear
- Copy result
- Model status indicator

Left pane states:

- Empty input
- Analyzing
- Analysis complete with highlights
- Pseudonymized preview generated immediately after analysis
- Error state

Right pane states:

- No findings yet
- Findings grouped by type
- Editable replacement rows
- Disabled rows stay visible but muted

### Highlight Behavior

- Different subtle highlight colors by sensitive type
- Hovering a highlight should reveal type and replacement
- Clicking a highlight should focus the corresponding replacement group
- Disabled replacement groups should remove or mute highlights

## 10. Tauri Commands

Initial command surface:

```rust
#[tauri::command]
async fn analyze_text(text: String) -> Result<AnalysisResult, AppError>;

#[tauri::command]
async fn apply_replacements(text: String, groups: Vec<ReplacementGroup>) -> Result<String, AppError>;

#[tauri::command]
async fn create_manual_finding(text: String, start: usize, end: usize, type_: SensitiveType) -> Result<Finding, AppError>;

#[tauri::command]
async fn recompute_analysis(text: String, findings: Vec<Finding>, groups: Vec<ReplacementGroup>) -> Result<AnalysisResult, AppError>;

#[tauri::command]
async fn get_model_status() -> Result<ModelStatus, AppError>;

#[tauri::command]
async fn get_model_download_status() -> Result<ModelDownloadStatus, AppError>;

#[tauri::command]
async fn start_model_download() -> Result<ModelDownloadStatus, AppError>;

#[tauri::command]
async fn pause_model_download() -> Result<ModelDownloadStatus, AppError>;

#[tauri::command]
async fn start_model_service() -> Result<ModelStatus, AppError>;

#[tauri::command]
async fn stop_model_service() -> Result<(), AppError>;

#[tauri::command]
async fn get_license_status() -> Result<LicenseStatus, AppError>;

#[tauri::command]
async fn activate_license(license_key: String) -> Result<LicenseStatus, AppError>;
```

The frontend should call `analyze_text`, receive findings and replacement groups, show the backend-generated initial pseudonymized preview, and keep an optimistic preview in sync with replacement edits. Manual marking should call `create_manual_finding`, append the returned finding, then call `recompute_analysis` with the explicit current findings and groups. `Copy result` should call `apply_replacements` first and copy the backend-confirmed result.

## 11. Python Sidecar Service

### Responsibilities

- Locate local Hugging Face model cache
- Load Qwen 1.7B model and tokenizer
- Expose `/status`
- Expose `/analyze`
- Return strict JSON
- Avoid logging user text

### Suggested API

```http
GET /status

POST /analyze
Content-Type: application/json

{
  "chunks": [
    {"chunkIndex": 0, "text": "..."}
  ]
}
```

Response:

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

### Model Discovery

The service should first try:

1. Explicit app config path
2. `PSEUDO_MODEL_PATH`
3. App-managed model directory
4. Hugging Face cache lookup for likely Qwen 1.7B model names
5. First-run model download prompt with clear size, privacy, and license notes
6. Manual setup error with clear UI message

Do not silently download models. The first-run experience should offer a guided download, show progress, validate the downloaded files, and then run analysis fully locally. The app should remember the installed model path and support replacing or deleting the local model from settings.

### Model Download Manager

For the lawyer-facing demo and future sales flow, keep the app installer small and download the model after install:

- show a first-run setup screen when no usable model is found
- explain that the model is downloaded once and future document analysis stays local
- show model name, approximate size, destination folder, and expected disk requirement
- support resume after interruption where the hosting source supports it
- validate the model with a checksum or manifest before use
- store the model under an app-managed data directory by default
- allow advanced users to choose an existing local model path
- avoid sending pasted/user text during setup, activation, or update checks
- verify commercial redistribution and hosted-download rights for the selected model before distributing outside private demos

## 12. Commercial Demo, Trial, and Free Version

The sales/demo experience should optimize for trust: install quickly, make setup understandable, and avoid sending client text anywhere.

Recommended packaging:

- Small installer that includes the app shell, deterministic detectors, manual marking, and model download manager.
- First-run guided setup that downloads the local model only after explicit user approval.
- Full local processing after the model is installed.
- Clear status: `Deterministic only`, `Downloading model`, `Local model ready`, `Trial expired`, or `Licensed`.

Recommended trial/free strategy:

- Free version: deterministic detectors, manual marking, replacement review, and copy/export for short text. This is useful forever and demonstrates privacy even without the model.
- Trial version: time-limited full LLM-assisted experience, such as 14 days, with no document upload and no watermark in copied text. This is best for a lawyer evaluating real workflows.
- Paid version: unlimited local LLM-assisted analysis, commercial support, model/settings management, and later document-format support.

Licensing should be privacy-preserving. Activation may contact a license server with license metadata and device/app identifiers, but never pasted text, extracted findings, replacement maps, or pseudonymized results. The app should continue to offer deterministic/manual functionality when offline or unlicensed.

## 13. Privacy and Security Requirements

- All analysis must run locally.
- No analytics or telemetry in MVP.
- Do not persist pasted text unless the user explicitly saves a project later.
- Do not log user text in Rust, Python, or frontend console.
- Do not send pasted text, findings, replacement maps, or pseudonymized output during license activation, model download, or update checks.
- Sidecar service should bind only to `127.0.0.1`.
- Use a random local port or authenticated local token if the sidecar exposes HTTP.
- Clear in-memory state when the user clicks `Clear`.
- Document that the app assists pseudonymization but does not guarantee legal anonymization.

## 14. Project Structure

Recommended initial structure:

```text
pseudo/
  package.json
  vite.config.ts
  index.html
  src/
    main.tsx
    App.tsx
    components/
      EditorPane.tsx
      FirstRunSetup.tsx
      ReplacementPanel.tsx
      SettingsPanel.tsx
      Toolbar.tsx
      HighlightedText.tsx
    lib/
      analysisTypes.ts
      replacement.ts
      chunkTextForAnalysis.ts
      tauriApi.ts
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/
      main.rs
      analysis.rs
      license.rs
      model_download.rs
      model_service.rs
      rules.rs
  sidecar/
    model_service.py
    requirements.txt
  docs/
    implementation-plan.md
```

This repository currently contains the plan at the root. Once the app scaffold exists, move or copy this document to `docs/implementation-plan.md`.

## 15. Testing Plan

### Unit Tests

Frontend:

- text chunking preserves offsets
- grouping repeated findings
- suggested replacement numbering
- optimistic pseudonymized preview generation after analysis
- disabled replacements are skipped
- selection-to-manual-finding UI behavior

Rust:

- regex detection for structured sensitive info
- model output validation
- model download status and checksum/manifest validation
- exact surface-form matching from LLM output to source ranges
- overlap resolution
- canonical grouping and replacement application from end to start
- manual finding validation for selected ranges
- license status and activation state handling without user text
- sidecar status handling

Python:

- response schema validation
- JSON extraction from model output
- model unavailable error

### Integration Tests

- paste sample text
- run deterministic-only analysis
- show highlights
- mark selected text as sensitive
- edit replacement
- verify pseudonymized preview updates automatically
- copy backend-confirmed final result
- first-run model download completes and enables local LLM analysis
- expired/unlicensed state falls back to deterministic/manual functionality

### Manual Test Text

```text
Jane Doe from ACME Health emailed john.smith@example.com on 12 March 2025.
Her patient ID is PT-44921 and she lives in Zurich.
Jane Doe, the senior claims manager, later called +41 44 123 45 67 about invoice INV-2025-991.
```

Expected groups:

- Jane Doe -> `[PERSON_1]`
- ACME Health -> `[ORG_1]`
- john.smith@example.com -> `[EMAIL_1]`
- 12 March 2025 -> `[DATE_1]`
- PT-44921 -> `[ID_1]`
- Zurich -> `[LOCATION_1]`
- senior claims manager -> `[ROLE_1]`
- +41 44 123 45 67 -> `[PHONE_1]`
- INV-2025-991 -> `[ID_2]`

## 16. Build Phases

### Phase 1: App Scaffold and Deterministic MVP

- Create Tauri + React + TypeScript app
- Build two-pane UI
- Implement paste/edit text area
- Implement deterministic detectors for email, phone, URL, dates, and ID-like values
- Implement canonical Rust grouping and range-based replacement application
- Implement highlighting and replacement panel
- Implement immediate range-based pseudonymized preview
- Keep source highlights, replacement rows, and pseudonymized result synchronized
- Add manual "mark selected text as sensitive"
- Add unit tests for replacement logic

Deliverable: usable app without LLM dependency.

### Phase 2: Local LLM Sidecar

- Add Python sidecar service
- Load local Qwen 1.7B from app-managed download, configured path, or Hugging Face cache
- Add first-run model download UI with progress and clear privacy copy
- Add checksum or manifest validation before model use
- Implement chunk batching and strict JSON prompt
- Add Rust command to start/status/check sidecar
- Merge LLM findings with deterministic findings
- Add model status UI

Deliverable: local LLM-assisted detection with guided model setup.

### Phase 3: Review Quality and UX Polish

- Add click-to-focus between highlight and replacement row
- Add type filters
- Add confidence display or review badges
- Add "ignore finding" action
- Add explicit "reset to suggested replacements" action
- Improve chunk boundary selection

Deliverable: practical review workflow.

### Phase 4: Packaging

- Bundle Python sidecar or provide managed runtime setup
- Keep installer small by excluding the model from the app bundle
- Add model download/update/delete settings
- Add model path settings screen
- Add privacy-preserving trial/license activation
- Add platform-specific packaging notes for macOS, Windows, and Linux
- Validate offline startup
- Validate no user text appears in logs

Deliverable: installable cross-platform desktop app suitable for lawyer-facing demos.

### Phase 5: Advanced Features

- Export pseudonymization map
- Import/export replacement presets
- Project/session save with explicit user consent
- Support structured documents
- Paid team or firm license management
- Optional stronger local NER model or fine-tuned classifier
- Optional native Rust inference runtime

## 17. Key Risks and Mitigations

### LLM Output Is Not Reliably Structured

Mitigation:

- Keep deterministic detectors
- Use strict JSON prompt
- Validate all model output
- Discard invalid spans
- Show warnings rather than failing the whole analysis

### Pseudonymization May Miss Sensitive Data

Mitigation:

- Present the app as assisted review
- Add manual marking quickly
- Prefer conservative highlighting
- Provide visible warning when model is unavailable

### Packaging Python and Transformers Is Heavy

Mitigation:

- Keep the app installer small and download the model after install
- MVP can assume developer/local Python environment until the demo packaging pass
- Later evaluate native inference or a bundled sidecar
- Keep the sidecar API isolated so runtime can be swapped

### Model Download or License Flow Hurts Trust

Mitigation:

- Require explicit user approval before model download
- Show model size, destination, and local-only analysis promise before setup
- Never send user text during setup, activation, or update checks
- Keep deterministic/manual functionality available without activation
- Verify model license and hosted-download rights before public/commercial distribution

### Replacing Text Naively Can Corrupt Output

Mitigation:

- Apply only validated character ranges
- Sort ranges descending
- Use Rust as the canonical replacement engine
- Confirm the final copied/exported result through the backend
- Keep tests around overlapping and repeated spans

## 18. Immediate Next Steps

1. Scaffold Tauri + React + TypeScript project.
2. Implement shared TypeScript types and Rust-side grouping/replacement utilities.
3. Build the source editor, replacement list, and auto-updating optimistic result preview with mock findings.
4. Add deterministic detectors and wire them through a Tauri command.
5. Add manual selected-text marking.
6. Add unit tests for grouping, overlap resolution, manual findings, and replacement.
7. Add first-run model setup UI state with placeholder download/status behavior.
8. Add the Python sidecar after the deterministic workflow feels solid.
