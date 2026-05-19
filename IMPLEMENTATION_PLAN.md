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

The user expects a local Qwen 1.7B model available through Hugging Face. The plan should support two runtime paths:

1. Primary MVP path: sidecar Python service using `transformers`
   - Easiest path to Hugging Face model loading
   - Can use local cache without network
   - Keeps Rust/Tauri integration simple through localhost HTTP or stdio
2. Later native path: Rust-side inference via `candle`, `llama.cpp`, or ONNX
   - Better packaging story
   - Lower operational complexity after model format is settled

For the first build, use the Python sidecar because it reduces risk and gives faster iteration on prompts and parsing.

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

`pseudonymizedText` is a derived value produced from the current text and enabled replacement groups. The backend may return the first generated preview, but the frontend should also recompute it whenever the user edits a replacement, disables a group, or re-enables a group.

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

Apply replacements automatically after every completed analysis and after every user edit to the replacement map. Apply by sorted character ranges, from end to start, not by naive global string replacement. This prevents accidental changes to text outside confirmed spans and preserves offsets during replacement.

Provide an optional later feature: "replace all exact matches" for user-approved recurring text missed by the model.

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
async fn get_model_status() -> Result<ModelStatus, AppError>;

#[tauri::command]
async fn start_model_service() -> Result<ModelStatus, AppError>;

#[tauri::command]
async fn stop_model_service() -> Result<(), AppError>;
```

Replacement application can live in the frontend initially because it is deterministic and easy to test. Move it to Rust later if shared validation or export features require it.

The frontend should call `analyze_text`, receive findings and replacement groups, generate the initial pseudonymized preview immediately, and then keep that preview in sync with replacement edits.

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
3. Hugging Face cache lookup for likely Qwen 1.7B model names
4. Manual setup error with clear UI message

Do not download models automatically in the first version. The user expects the model to already exist locally.

## 12. Privacy and Security Requirements

- All analysis must run locally.
- No analytics or telemetry in MVP.
- Do not persist pasted text unless the user explicitly saves a project later.
- Do not log user text in Rust, Python, or frontend console.
- Sidecar service should bind only to `127.0.0.1`.
- Use a random local port or authenticated local token if the sidecar exposes HTTP.
- Clear in-memory state when the user clicks `Clear`.
- Document that the app assists pseudonymization but does not guarantee legal anonymization.

## 13. Project Structure

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
      ReplacementPanel.tsx
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
      model_service.rs
      rules.rs
  sidecar/
    model_service.py
    requirements.txt
  docs/
    implementation-plan.md
```

This repository currently contains the plan at the root. Once the app scaffold exists, move or copy this document to `docs/implementation-plan.md`.

## 14. Testing Plan

### Unit Tests

Frontend:

- text chunking preserves offsets
- grouping repeated findings
- suggested replacement numbering
- automatic pseudonymized preview generation after analysis
- applying replacements from end to start
- disabled replacements are skipped

Rust:

- regex detection for structured sensitive info
- model output validation
- exact surface-form matching from LLM output to source ranges
- overlap resolution
- sidecar status handling

Python:

- response schema validation
- JSON extraction from model output
- model unavailable error

### Integration Tests

- paste sample text
- run deterministic-only analysis
- show highlights
- edit replacement
- verify pseudonymized preview updates automatically
- copy final result

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

## 15. Build Phases

### Phase 1: App Scaffold and Deterministic MVP

- Create Tauri + React + TypeScript app
- Build two-pane UI
- Implement paste/edit text area
- Implement deterministic detectors for email, phone, URL, dates, and ID-like values
- Implement highlighting and replacement panel
- Implement immediate range-based pseudonymized preview
- Keep source highlights, replacement rows, and pseudonymized result synchronized
- Add unit tests for replacement logic

Deliverable: usable app without LLM dependency.

### Phase 2: Local LLM Sidecar

- Add Python sidecar service
- Load local Qwen 1.7B from configured path or Hugging Face cache
- Implement chunk batching and strict JSON prompt
- Add Rust command to start/status/check sidecar
- Merge LLM findings with deterministic findings
- Add model status UI

Deliverable: local LLM-assisted detection.

### Phase 3: Review Quality and UX Polish

- Add click-to-focus between highlight and replacement row
- Add type filters
- Add confidence display or review badges
- Add manual "mark selected text as sensitive"
- Add "ignore finding" action
- Add explicit "reset to suggested replacements" action
- Improve chunk boundary selection

Deliverable: practical review workflow.

### Phase 4: Packaging

- Bundle Python sidecar or provide managed runtime setup
- Add model path settings screen
- Add platform-specific packaging notes for macOS, Windows, and Linux
- Validate offline startup
- Validate no user text appears in logs

Deliverable: installable cross-platform desktop app.

### Phase 5: Advanced Features

- Export pseudonymization map
- Import/export replacement presets
- Project/session save with explicit user consent
- Support structured documents
- Optional stronger local NER model or fine-tuned classifier
- Optional native Rust inference runtime

## 16. Key Risks and Mitigations

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

- MVP can assume developer/local Python environment
- Later evaluate native inference or a bundled sidecar
- Keep the sidecar API isolated so runtime can be swapped

### Replacing Text Naively Can Corrupt Output

Mitigation:

- Apply only validated character ranges
- Sort ranges descending
- Keep tests around overlapping and repeated spans

## 17. Immediate Next Steps

1. Scaffold Tauri + React + TypeScript project.
2. Implement shared TypeScript types and replacement utilities.
3. Build the source editor, replacement list, and auto-updating result preview with mock findings.
4. Add deterministic detectors and wire them through a Tauri command.
5. Add unit tests for grouping and replacement.
6. Add the Python sidecar after the deterministic workflow feels solid.
