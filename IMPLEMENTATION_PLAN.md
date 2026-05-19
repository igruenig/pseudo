# Pseudonymization App Implementation Plan

## 1. Product Goal

Build a cross-platform desktop app with Tauri for professionals who handle confidential text and want to use AI without sending text or pseudonymization data off-device. Lawyers are the initial launch wedge, but the product should remain horizontal enough for therapists, HR teams, consultants, executives, researchers, and other privacy-sensitive professionals. The app should analyze text in offset-preserving chunks, identify directly or indirectly identifying spans, group recurring occurrences, suggest generic replacements, and immediately show a pseudonymized result that the user can review before copying.

The first version should prioritize local privacy, transparent review, predictable replacement behavior, and a strong first impression. A new user should be able to see useful pseudonymization results before model setup, licensing, or configuration, with the privacy promise reinforced by observable local pseudonymization behavior rather than heavy explanatory copy. The app can automatically apply suggested replacements after analysis, but the user must always be able to inspect and adjust the replacement map.

Treat the deterministic/manual app as the real MVP, not as a degraded fallback. The local LLM should arrive later as an accuracy upgrade after the user has already seen a useful, local, understandable workflow.

Privacy invariant: user content never leaves the machine. No source text, detected findings, replacement map, pseudonymized output, saved project content, imported document content, or manually marked text may ever be sent to any network service. Pseudonymization always runs locally, including when the optional model is installed. The app may use the network only for explicit non-content operations such as downloading a model, license activation, update checks, or enterprise/admin metadata sync, and those operations must never include user content or pseudonymization artifacts.

## 2. Core User Flow

### First-Run Experience

The first launch should open into a usable deterministic/manual app, not a setup gate. If no model is installed, the user can still experience the core product immediately.

1. App opens with a realistic confidential-work sample already analyzed. The initial sample can resemble a lawyer's intake note or settlement message, but product UI copy should not mention legal work, law firms, or lawyers.
2. The first visible state shows highlighted original text, populated replacement groups, a rendered pseudonymized result, readiness summary, enabled `Copy result`, and clear action.
3. The primary first-run action is `Try your own text`, which clears the sample and focuses the empty editor.
4. The first-run trust strip uses use-case language: `Use AI on confidential work`, `No text or results are sent`, `Nothing saved`, and a neutral status such as `Basic detection active`. Avoid `deterministic mode` and avoid model-status language on the first screen.
5. After the pre-analyzed sample appears, show a subtle proof of locality such as `Demo ready locally · no text or results sent`.
6. The app does not show license activation, account creation, settings, model download, model status, upgrade banners, or model setup as primary first-run actions.
7. The UI explains model setup only after the user has successfully analyzed their own text at least once: installing the local model improves detection for names, organizations, roles, and context-sensitive spans.

The built-in sample must ship as static fixtures: sample text, findings, replacement groups, replacement memory, pseudonymized output, and expected text hash. No analysis runs on launch. This keeps the first paint fast and reliable, and avoids showing fake timing. User-provided text in the no-model state should use deterministic detectors plus manual marking until the local model is enabled.

### First-Impression Requirements

- Fresh install with no model, no license, and no configuration must open directly into a successful analyzed sample state.
- The pre-analyzed sample should be ready at launch from static bundled fixtures; no detector run is required before first paint. The underlying deterministic analysis target for user-initiated analysis remains under 500 ms on normal desktop hardware.
- The first screen should lead with before/after transformation: original text and pseudonymized text should be directly comparable at a glance, with the replacement panel secondary.
- `Copy result` should be enabled for the analyzed sample and disabled only for `EMPTY`, `DIRTY_NEEDS_ANALYSIS`, `ANALYZING`, or invalid states.
- Manual marking should be visible enough to communicate user control, even if the new user does not use it during the sample flow.
- Secondary review controls such as type filters, confidence badges, ignore actions, and reset-to-suggested should be progressively disclosed rather than visible by default on first launch.
- Warnings should be concise and actionable. Avoid confidence-killing global disclaimers before the user has seen the workflow.
- Any model-download or license prompt should appear only after a successful analysis on the user's own text, not after the built-in sample alone.
- When the user clears the sample, the empty editor placeholder should say: `Paste confidential text. No text or results are sent.`
- Copy confirmation should be a transient toast, not a modal, with specific reassurance: `Pseudonymized text copied · no text or results sent`.

### Working Flow

1. User opens the app into the already-analyzed sample or clicks `Try your own text`.
2. User pastes text into the editor.
3. The pasted or edited text enters `DIRTY_NEEDS_ANALYSIS`; analysis does not run automatically in Phase 1.
4. User clicks `Analyze`.
5. App splits the text into offset-preserving chunks and sends them to the available local detection pipeline.
6. Sensitive spans are highlighted inline.
7. A side panel lists unique detected entities grouped by canonical text and type.
8. Each list item has:
   - detected value
   - sensitive info type
   - occurrence count
   - suggested pseudonym
   - editable replacement field
   - enable/disable toggle
9. App immediately applies enabled suggested replacements and shows the pseudonymized result.
10. User reviews and edits replacements.
11. The pseudonymized preview updates immediately after each replacement edit or toggle, but only while the current source text still matches the analyzed text.
12. Before copy, the app shows a concise readiness summary, such as `Ready to copy`, `9 findings`, `7 replacements enabled`, `2 need review`, or `Manual review recommended`.
13. User copies the final text and sees `Pseudonymized text copied · no text or results sent`.

### Analysis State Model

Phase 1 uses explicit analysis. Pasting or typing should not trigger analysis automatically. Debounced automatic analysis can be considered later only if it remains local, fast, and does not make review state feel jumpy.

State names:

- `SAMPLE_READY`: built-in sample is already analyzed; highlights, replacement panel, preview, and `Copy result` are enabled.
- `EMPTY`: no source text; no findings; `Copy result` disabled.
- `DIRTY_NEEDS_ANALYSIS`: source text has changed since the last successful analysis; `Analyze` is primary; `Copy result` disabled.
- `ANALYZING`: backend analysis is running; source editing may remain possible, but completion must be ignored if the analyzed text hash no longer matches current text.
- `ANALYZED_READY`: current source text matches `analysis.textHash`; highlights, replacement edits, live preview, and `Copy result` are enabled.
- `ERROR`: analysis failed; prior results may be shown only if clearly marked stale.

Dirty-state UI:

- When source text changes after analysis, keep previous highlights and preview visible but dimmed and labeled `Needs re-analysis`.
- Replacement rows remain visible but disabled until re-analysis, except `Reset replacements`.
- `Copy result` is disabled in `DIRTY_NEEDS_ANALYSIS`.
- Readiness summary should say `Review paused · analyze again`.
- Manual marking is disabled in `DIRTY_NEEDS_ANALYSIS`; the user must analyze current text first so selected ranges are validated against the current source.

Backend sync rule:

- Every analysis result carries `textHash`.
- Every preview/copy/apply request must include the current source text and expected `textHash`.
- If hashes do not match, backend commands return a stale-analysis error instead of producing copyable output.

### Voice and Copy Principles

- Lead with the use case, not the implementation: help people use AI on confidential work without exposing identities.
- Use plain language over technical jargon, and prefer observable behavior over broad privacy claims.
- Never apologize for the Free tier, make the app feel incomplete, or pressure the user to upgrade.

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

### Open-Source-Ready Core Boundary

Build the deterministic engine as if it may be open-sourced later, but do not open-source it during the first product phases. The goal is to make open-sourcing a switch that can be flipped after paying customers, stable architecture, and the Phase 3 local LLM integration prove the product shape.

- `pseudo-core` owns deterministic detectors, chunking, grouping, range-based replacement, overlap resolution, manual-finding validation, and shared data types.
- `pseudo-core` must be a pure Rust library with no Tauri dependency, no license checks, no model download code, no model runtime orchestration, no telemetry, no auto-update logic, and no network calls of any kind.
- The Tauri binary crate owns app commands, UI-facing orchestration, license activation, license server URLs, signing-key handling, model download, model runtime orchestration, packaging integration, and update mechanisms.
- `pseudo-core` remains free of `llama-cpp-2`, model files, and inference code. The `llama-cpp-2` dependency lives only in the closed Tauri app crate or a dedicated closed `pseudo-runtime` crate inside the app workspace.
- Every dependency in `pseudo-core` must be permissively licensed: MIT, Apache-2.0, BSD, ISC, or MPL-2.0. Do not allow GPL or AGPL dependencies in the core.
- Before adding `phonenumber` or any other parsing dependency to `pseudo-core`, verify its license with `cargo deny` and keep it inside the permissive-license policy.
- Add `cargo deny` in CI from day one to check dependency licenses and catch accidental policy drift while the codebase is still small.
- Prefer Apache-2.0 for a future `pseudo-core` release because it includes an explicit patent grant.

Ship a small `pseudo-cli` wrapper around `pseudo-core`, even before any open-source release. A command such as `pseudo-cli analyze < input.txt > output.json` gives integration tests a UI-free path, demonstrates the local deterministic engine, and later becomes a runnable audit artifact for security teams.

### Frontend

- React + TypeScript
- State management with local React state first; introduce Zustand only if state becomes awkward
- Split frontend utility code into `src/lib/core` for shareable types and pure-function helpers, and `src/lib/app` for Tauri-specific glue, settings, licensing, model status, and other closed app integration.
- Text editor:
  - MVP: controlled textarea plus overlay-based highlighting
  - Later: CodeMirror 6 or TipTap if richer text selection/review is needed
- Styling:
  - CSS modules or plain scoped CSS
  - Avoid heavy UI frameworks for the first version

### Local Model Runtime

The app uses `llama.cpp` (MIT) as the inference engine, via the `llama-cpp-2` Rust bindings (`utilityai/llama-cpp-rs`), to run Qwen3-1.7B in GGUF format (Apache-2.0). Inference runs in-process inside the Tauri app crate or a dedicated closed `pseudo-runtime` crate. There is no Python runtime, virtualenv, localhost HTTP server, or out-of-process model service.

Quantization:

- Use Q4_K_M as the default download target. Current target: `ggml-org/Qwen3-1.7B-GGUF:Q4_K_M`, unless the Qwen namespace publishes an official Q4_K_M file before implementation.
- Allow advanced users to point at an existing local Q8_0 GGUF file for higher fidelity.
- Run a small internal eval before public release to verify Q4_K_M quality on realistic confidential-text span extraction. If Q4_K_M misses material spans that Q8_0 catches, prefer Q8_0 despite the larger download.

Backend strategy for v1:

- macOS: Metal backend, especially for Apple Silicon.
- Windows/Linux: CPU backend in the default installer.
- CUDA or Vulkan builds are separate optional installers after real customer demand; do not bundle CUDA in the default Windows/Linux installer because of size and packaging complexity.

Licensing notes to retain in product docs:

- Qwen3-1.7B and Qwen GGUF weights are Apache-2.0. If the selected Q4_K_M file comes from `ggml-org/Qwen3-1.7B-GGUF`, preserve that repo's upstream license metadata as well as the original Qwen license reference.
- `llama.cpp` is MIT. Include its license in `third-party-notices.md`.
- `llama-cpp-2` is MIT OR Apache-2.0. Include it in `third-party-notices.md`.

## 5. High-Level Architecture

```text
React UI
  |
  | Tauri command: analyze_text(request_id, text)
  v
Rust backend (app crate)
  |
  | runs deterministic detectors (pseudo-core)
  | optionally calls in-process llama.cpp via llama-cpp-2
  |   with offset-preserving chunk batches
  v
llama.cpp inference (in-process, no IPC)
  |
  | loads local Qwen3-1.7B GGUF from app-managed cache
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
  aliases?: string[];
  aliasOfGroupId?: string;
};
```

### Analysis Result

```ts
type AnalysisResult = {
  textHash: string;
  findings: Finding[];
  groups: ReplacementGroup[];
  pseudonymizedText: string;
  replacementMemory: ReplacementMemory;
  warnings: string[];
};
```

### Replacement Memory

```ts
type ReplacementMemoryEntry = {
  type: SensitiveType;
  normalizedOriginal: string;
  replacement: string;
  enabled: boolean;
  firstAssignedAt: string;
};

type ReplacementMemory = {
  entries: ReplacementMemoryEntry[];
};
```

`pseudonymizedText` is a derived value produced from the current text and enabled replacement groups. The Rust backend should be the canonical owner of validation, grouping, overlap resolution, and range-based replacement. The frontend may recompute an optimistic live preview for responsiveness only while in `ANALYZED_READY`, but final copy/export actions should use backend-confirmed output.

### Preview Sync Strategy

Phase 1 uses pure-frontend optimistic preview for replacement edits.

- Replacement field edits and enable/disable toggles update the preview synchronously in frontend state. Do not debounce these updates; the local range-based algorithm should be fast enough for each keystroke.
- Do not call backend recomputation on every replacement edit or toggle.
- Backend recomputation happens for `analyze_text`, manual finding changes, explicit reset/recompute actions, and final `Copy result`.
- `Copy result` calls `apply_replacements` with current text, expected text hash, and current replacement groups, then copies only the backend-confirmed output.
- If backend-confirmed output differs from the optimistic preview, replace the preview with the backend result, copy the backend result, and show a non-blocking note such as `Copied backend-verified result`.
- If backend returns a stale hash error, do not copy. Move to `DIRTY_NEEDS_ANALYSIS` and show `Analyze again before copying`.
- Analysis-style requests carry a monotonically increasing frontend `requestId`. The frontend must ignore any response whose `requestId` is older than the latest request for the current text hash.

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

#### Deterministic Rule Contract for `pseudo-core/src/rules.rs`

Keep Phase 1 deterministic rules conservative and documented. The goal is predictable useful detection, not broad NER coverage. Anything outside these rules should be handled by manual marking or, later, the local LLM.

Language and locale scope:

- Phase 1 deterministic detectors should support English and German-language text.
- Rules should be language-aware where labels or month names matter, but should not attempt general German/English NER without the local LLM.
- Preserve the exact source text and offsets for all languages; do not transliterate German umlauts or normalize locale-specific punctuation in detected spans.

Email:

- Accept ordinary address forms with a local part, `@`, domain labels, and a 2+ character TLD, using a compiled regex such as `(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b`.
- Reject matches embedded inside longer tokens.

URL:

- Accept `http://` and `https://` URLs until whitespace or a closing bracket/quote.
- Accept bare `www.` URLs only when followed by at least one dot and a 2+ character TLD.
- Do not detect arbitrary bare domains in Phase 1 because false positives are high.

Phone:

- Use the `phonenumber` crate rather than hand-rolled regexes for accepted phone formats.
- Accept international numbers with explicit country codes when `phonenumber` parses and validates them, including `+41 44 123 45 67` and `+49 (0)30 1234-5678` after normalizing optional national trunk markers such as `(0)`.
- Accept German and Swiss national-format numbers only when there is clear country context. Initial context signals: user-selected locale, app locale, or explicit nearby country/city terms such as `Germany`, `Deutschland`, `Switzerland`, `Schweiz`, `Suisse`, `Zurich`, `Zürich`, `Berlin`, or `Munich`/`München`.
- Accept North American numbers with area code when they parse as US/CA numbers, including `(415) 555-0123`, `415-555-0123`, and `415 555 0123`.
- Reject short or bare local numbers without a country or area code, including `5551234`; users can mark these manually.
- Reject numeric strings that are more likely IDs, dates, amounts, or short codes unless `phonenumber` validates them with clear country context.

Date:

- Accept unambiguous English month-name dates: `12 March 2025`, `March 12, 2025`, `12 Mar 2025`, and `Mar 12, 2025`.
- Accept unambiguous German month-name dates: `12. März 2025`, `12 März 2025`, `12. Mrz. 2025`, `12 Oktober 2025`, `12. Okt. 2025`, and the corresponding full German month names.
- Accept ISO dates: `2025-03-12`.
- Accept dotted or slashed numeric dates only when unambiguous:
  - `13.03.2025` and `03/13/2025` are valid because one side is greater than 12.
  - `12.03.2025` and `03/12/2025` are ambiguous and should not be auto-detected in Phase 1.
- Do not normalize date values in Phase 1; preserve the exact detected surface form.

ID numbers:

- Accept contextual IDs when a nearby English or German label appears within roughly 24 characters before the token.
- English labels include `id`, `case`, `matter`, `client`, `patient`, `account`, `invoice`, `ref`, `reference`, `ticket`, `claim`, and `policy`.
- German labels include `akte`, `aktenzeichen`, `az`, `mandant`, `mandantin`, `kunde`, `kundin`, `konto`, `rechnung`, `rechnungsnummer`, `ref`, `referenz`, `vorgang`, `ticket`, `schaden`, `police`, `vertragsnummer`, `patient`, and `patientin`.
- Accept token shape `\b[A-Z]{2,}[A-Z0-9]*(?:-[A-Z0-9]+)+\b` when the token contains at least one digit, such as `PT-44921` or `INV-2025-991`.
- Reject lowercase hyphenated words and adjective phrases such as `gdpr-compliant`.
- Reject known common false positives such as `COVID-19` unless a contextual ID label is present.
- Reject generic all-caps words without digits.

Acceptance criteria before implementing `rules.rs`:

- Add positive fixtures for the first-run sample, Swiss phone, German phone with `(0)`, US phone, English month-name dates, German month-name dates, ISO dates, English contextual IDs, German contextual IDs, email, and URL.
- Add negative fixtures for `5551234`, `12.03.2025`, `03/12/2025`, `COVID-19`, `GDPR-compliant`, bare domains without scheme/`www`, and random hyphenated phrases.
- Add bilingual fixture paragraphs in English and German that mix email, phone, date, URL, and contextual ID detections.
- Every deterministic finding must preserve exact source offsets and exact source text.

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

The Rust backend should validate model output before passing it to the UI:

- discard findings whose text does not exist in the source chunk
- clamp confidence to `0..1`
- reject unknown types
- resolve overlapping spans deterministically
- preserve original casing and whitespace

### Step 5: Canonical Overlap Resolution

All deterministic, manual, and LLM findings must flow through one canonical resolver in `pseudo-core/src/overlap.rs`. The resolver receives validated candidate findings with exact source offsets and returns a sorted, non-overlapping list.

Candidate metadata:

- `source`: `MANUAL`, `DETERMINISTIC`, or `LLM`
- `type`
- `start`
- `end`
- `text`
- `confidence`
- optional detector/rule name

Priority function:

1. Reject invalid ranges where `start >= end` or where the candidate text does not exactly equal `sourceText[start..end]`.
2. Sort candidates by `start` ascending, then `end` descending, then priority score descending.
3. Build connected overlap clusters. Two candidates are in the same cluster when their ranges overlap by at least one character, directly or through another overlapping candidate.
4. For each cluster, choose winners greedily by priority score. Add a candidate if it does not overlap any already chosen winner in that cluster.
5. Sort final winners by `start` ascending.

Priority score, highest first:

1. source priority: `MANUAL` > `DETERMINISTIC` > `LLM`
2. type priority for deterministic ties: `EMAIL` > `URL` > `PHONE` > `ID_NUMBER` > `DATE` > `PERSON_NAME` > `ORGANIZATION` > `ROLE_OR_POSITION` > `LOCATION` > `OTHER_SENSITIVE`
3. longer span wins when one candidate fully contains another
4. higher confidence wins
5. earlier start wins
6. later end wins
7. stable candidate id wins as final tie-breaker

Required behaviors:

- Manual findings override overlapping deterministic or LLM findings.
- Deterministic findings override overlapping LLM findings, including same-span type disagreement.
- Among deterministic findings, type priority resolves same-span or partial-overlap conflicts. For example, an email finding should win over a URL/domain finding that overlaps its domain.
- When source and type priority tie, longer contained spans win, so `Jane Doe` wins over `Jane`.
- Partial overlaps where neither span contains the other still resolve deterministically through the same score. The lower-scored candidate is discarded rather than trimmed.
- The resolver never mutates candidate ranges or splits findings.

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

Phase 1 co-reference scope:

- Group exact normalized matches only.
- Do not automatically group `Jane`, `Jane Doe`, `Ms. Doe`, or pronouns together in Phase 1.
- Rely on manual marking for aliases in Phase 1.
- Defer model-assisted alias/co-reference suggestions to Phase 3.
- Keep `aliases` and `aliasOfGroupId` in the data model for later reviewable alias grouping, but leave them unused by default in Phase 1.

Do not implement substring-containment grouping in Phase 1. It creates tempting false positives and requires parent/child review behavior that the first deterministic MVP does not need.

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

Suggested replacement numbering should be deterministic by first occurrence offset for newly seen groups. Existing session mappings take precedence over first-occurrence order.

### Re-Analysis Stability

Replacement choices must persist across re-analysis within the same unsaved session unless the user explicitly resets them.

- Maintain a session-local `ReplacementMemory` keyed by `(type, normalizedOriginal)`.
- On re-analysis, if a new group matches an existing memory entry, reuse its `replacement` and `enabled` state.
- For groups not present in memory, assign the next available number for that type based on first occurrence offset among only the newly seen groups.
- Do not renumber existing remembered replacements just because a new finding appears earlier in the edited text.
- If a remembered group disappears from the current text, keep its memory entry for the session so it can reappear without changing replacement.
- Provide an explicit `Reset replacements` action to clear memory and regenerate suggestions from the current analysis.

Example: if `Jane Doe` is first assigned `[PERSON_1]`, then the user edits text so `John Smith` appears before `Jane Doe`, `Jane Doe` remains `[PERSON_1]` and `John Smith` becomes the next available person replacement.

### Readiness Summary

The readiness summary is computed from analysis state, findings, replacement groups, and warnings. Use one deterministic function for the status text and counts.

Definitions:

- A finding `needsReview` when its type is `OTHER_SENSITIVE`, its confidence is below `0.70`, or it has a warning from validation/model parsing.
- A replacement group `needsReview` when any occurrence needs review, when the group replacement is empty/whitespace, or when an enabled group has an invalid replacement token.
- A manual finding does not need review merely because it is manual; it needs review only if its replacement group has no chosen replacement or has an invalid replacement.
- A disabled group is user intent, not `needsReview`; count it separately as `disabled`.
- Ignored findings, once that feature exists, are user intent and count separately as `ignored`, not `needsReview`.

Status rules:

- `EMPTY`: show `Paste text to begin`; `Copy result` disabled.
- `DIRTY_NEEDS_ANALYSIS`: show `Review paused · analyze again`; `Copy result` disabled.
- `ANALYZING`: show `Analyzing...`; `Copy result` disabled.
- `ERROR`: show a concise actionable error; `Copy result` disabled unless there is a clearly valid previous backend-confirmed result for unchanged text.
- `ANALYZED_READY` with zero findings: show `No findings`; `Copy result` enabled because copying unchanged text may still be useful.
- `ANALYZED_READY` with findings and `needsReviewCount = 0`: show `Ready to copy · {findingCount} findings · {enabledCount} replacements enabled`.
- `ANALYZED_READY` with `needsReviewCount > 0`: show `Manual review recommended · {needsReviewCount} need review · {enabledCount} replacements enabled`.
- Include disabled/ignored counts only as secondary details, e.g. `{disabledCount} disabled`, not as blockers.

Thresholds:

- Deterministic findings should generally use confidence `1.0`.
- LLM findings below `0.70` need review.
- LLM findings without confidence should default to `0.50` and need review.
- `OTHER_SENSITIVE` always needs review regardless of confidence.

### Applying Replacements

Apply replacements automatically in the preview after every completed analysis and after every user edit to the replacement map. The canonical implementation should live in Rust and apply by sorted character ranges, from end to start, not by naive global string replacement. The frontend mirrors that algorithm for optimistic preview only while in `ANALYZED_READY`. This prevents accidental changes to text outside confirmed spans and preserves offsets during replacement.

The frontend may run the same deterministic algorithm for immediate preview updates while in `ANALYZED_READY`, but the backend remains the source of truth. Before copying or exporting, ask the backend to recompute the pseudonymized result from the current source text, expected text hash, and replacement groups.

Provide an optional later feature: "replace all exact matches" for user-approved recurring text missed by the model.

### Manual Marking

Manual user markings are first-class findings. In the MVP, the user should be able to select text in the editor, choose `Mark sensitive`, pick a sensitive type, and create a validated finding from the exact selected range. The manual finding should flow through the same grouping, replacement, highlighting, and range-based replacement logic as deterministic and LLM findings.

## 9. Frontend Layout

### Main Screen

Use two related layouts:

First-launch demo layout:

- Lead with before/after transformation.
- Show the already-analyzed sample original and pseudonymized result directly beside each other on desktop or stacked on mobile.
- Keep replacement review visible but secondary, either below the before/after comparison or in a narrower side panel.
- Show `Copy result` enabled, `Try your own text` as the primary next action, and a subtle proof line such as `Demo ready locally · no text or results sent`.

Working layout after the user pastes their own text, edits the source, or interacts with replacements:

- Left pane: source text editor with highlighted identifying spans
- Right pane: replacement review list
- Under or beside the source editor, depending on available window width, show the pseudonymized result preview

The result preview should be populated immediately after analysis and update live as the replacement list changes only in `ANALYZED_READY`. Source text edits move the UI to `DIRTY_NEEDS_ANALYSIS` and freeze the prior preview as stale until the user clicks `Analyze` again.

Top toolbar:

- Analyze
- Clear
- Copy result
- Try your own text on first launch

Do not show a model status indicator, model setup affordance, activation prompt, or upgrade banner on first launch. After the user has analyzed their own text at least once, the toolbar or status area may expose model setup as a secondary action.

Secondary review controls such as type filters, confidence badges, ignore finding, and reset-to-suggested should be progressively disclosed through a `More` affordance or shown on the second analysis onward. They should not compete with the first screen's see, understand, copy path.

Left pane states:

- Empty input
- Analyzing
- Analysis complete with highlights
- Dirty / needs re-analysis with dimmed stale highlights
- Pseudonymized preview generated immediately after analysis
- Error state

The empty input placeholder should say: `Paste confidential text. No text or results are sent.`

Right pane states:

- No findings yet
- Findings grouped by type
- Editable replacement rows
- Disabled rows stay visible but muted
- Stale rows visible but disabled in `DIRTY_NEEDS_ANALYSIS`

### Highlight Behavior

- Different subtle highlight colors by sensitive type
- Hovering a highlight should reveal type and replacement
- Clicking a highlight should focus the corresponding replacement group
- Disabled replacement groups should remove or mute highlights

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
async fn recompute_analysis(request_id: String, text: String, expected_text_hash: String, findings: Vec<Finding>, groups: Vec<ReplacementGroup>, replacement_memory: ReplacementMemory) -> Result<AnalysisResult, AppError>;

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
async fn pause_model_download() -> Result<ModelDownloadStatus, AppError>;

#[tauri::command]
async fn get_license_status() -> Result<LicenseStatus, AppError>;

#[tauri::command]
async fn activate_license(license_key: String) -> Result<LicenseStatus, AppError>;

#[tauri::command]
async fn get_audit_log_status() -> Result<AuditLogStatus, AppError>;

#[tauri::command]
async fn record_analysis_event(event: AnalysisAuditEvent) -> Result<(), AppError>;
```

The frontend should call `analyze_text`, receive findings and replacement groups, show the backend-generated initial pseudonymized preview, and keep an optimistic preview in sync with replacement edits while in `ANALYZED_READY`. Manual marking should call `create_manual_finding`, append the returned finding, then call `recompute_analysis` with the explicit current findings, groups, session `ReplacementMemory`, and expected text hash. `Copy result` should call `apply_replacements` first and copy the backend-confirmed result. Backend commands that depend on a prior analysis must reject mismatched `expected_text_hash`. `request_id` is echoed back by the frontend command wrapper or response envelope so stale responses can be ignored.

`ModelStatus` should expose `{ loaded: boolean, modelPath?: string, quantization?: string, backend: "metal" | "cpu" | "cuda" | "vulkan", loadMs?: number, residentMemoryMb?: number }`. Model lifecycle is in-process and owned by the Rust app/runtime layer.

`LicenseStatus` should distinguish `Free`, `Trial`, `ProSubscription`, `ProPerpetual`, `Firm`, and `Enterprise`. Trial state should include an expiry timestamp and must transition to `Free` on expiry rather than locking the app. Subscription state should include renewal status/date. Perpetual state should include `maintenanceActive` and `updateEligibleUntil` so the app can keep running while only updates become gated. Firm and Enterprise states should expose only the entitlements needed by the app, not license-server internals.

`get_audit_log_status` and `record_analysis_event` support Firm and Enterprise tiers only. They must be license-gated and metadata-only: never source text, never finding surface forms, never replacement maps, and never pseudonymized output.

`AuditLogStatus` should include whether audit logging is licensed, enabled, locally stored, admin-sync enabled, and the current retention window. It should not expose content paths or any data derived from source text.

## 11. Local Model Service (In-Process)

Inference runs inside the Tauri app crate, or a dedicated closed `pseudo-runtime` crate, via `llama-cpp-2`. There is no separate process, no IPC, no localhost port, and no Python runtime.

Responsibilities:

- Locate the local GGUF file.
- Load the model on first LLM-assisted analysis, not at app launch, so cold start stays fast and memory remains free when deterministic/manual detection is enough.
- Support an explicit `load_model` warm-up action from settings.
- Unload after a configurable idle window, default 10 minutes, to free RAM.
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
4. Guided download prompt with clear size, privacy, and license notes, shown only after the user has analyzed their own text.
5. Manual setup error with clear UI message.

Do not silently download models. Model setup should be framed as an optional upgrade from deterministic/manual analysis, not as a blocker on first launch. The guided download should show progress, validate the downloaded files, and then run LLM-assisted analysis fully locally. The app should remember the installed model path and support replacing or deleting the local model from settings.

### Model Download Manager

For the initial professional-facing demo and future sales flow, keep the app installer small and download the model after install:

- open first launch into the already-analyzed deterministic/manual sample even when no model is found
- offer model setup from settings and a post-analysis upgrade prompt only after the user has analyzed their own text
- download `ggml-org/Qwen3-1.7B-GGUF:Q4_K_M`, roughly 1.3 GB, from Hugging Face by default, unless an official Qwen-namespace Q4_K_M target is available before implementation
- preserve and store the upstream `LICENSE` and any `NOTICE` files alongside the GGUF file
- explain that the model is downloaded once and future document analysis stays local
- explain what the model adds: better detection for names, organizations, roles, and context-sensitive spans
- show model name, quantization, approximate size, destination folder, and expected disk requirement
- support resumable download where the hosting source supports it
- validate the model with a checksum or manifest before use
- store the model under an app-managed data directory by default
- allow advanced users to choose an existing local Q8_0 or other compatible GGUF path
- never send user content or pseudonymization artifacts during setup, activation, update checks, model download, or admin sync

### Memory, Lifecycle, and Concurrency

- Budget roughly 1.5-2 GB resident memory for Q4_K_M Qwen3-1.7B with default KV-cache settings, then tune after measurement on reference hardware.
- Document the memory expectation in settings and validate available memory before loading the model.
- Keep the model loaded while LLM analysis is active.
- After the idle window expires, unload the model and show `Local model ready, unloaded` rather than implying a failure.
- If load or inference fails, fall back to deterministic/manual mode and show `Local model unavailable. Basic detection is still available.`
- Do not include user text, findings, prompts, or replacement maps in crash logs or crash UI. Crash reports, if added later, must be metadata-only and opt-in.
- Serialize analyses through a single `LlamaSession` behind a mutex for v1.
- Parallel chunk batching within one analysis is allowed; concurrent LLM analyses are not worth the complexity in the first version.

## 12. Commercial Demo, Trial, and Pricing

The sales/demo experience should optimize for trust and time-to-value for professionals who handle confidential text: install quickly, show a useful deterministic result immediately, make setup understandable, and avoid sending source text, findings, replacements, or pseudonymized output anywhere. Lawyers are the initial go-to-market wedge, but pricing and product language should remain horizontal.

Recommended packaging:

- Small installer that includes the app shell, deterministic detectors, manual marking, and model download manager.
- First launch opens into the working deterministic/manual app with an already-analyzed confidential-work sample, before/after preview, trust/status strip, replacement review, locality proof, and copy behavior.
- Guided local model setup is offered only after explicit user approval, preferably after the user has seen deterministic analysis results on their own text.
- Full local processing after the model is installed.
- Clear status in settings or post-own-text analysis states: `Basic detection active`, `Downloading model`, `Local model ready`, `Trial active`, `Trial expired`, `Pro`, `Firm`, or `Enterprise`.

### Pricing Tiers

- Free, forever: deterministic detectors, manual marking, replacement review, unlimited text length, unlimited use, and copy/export. Explicit non-goal: never make Free worse to push upgrades.
- Pro: `$12/month`, `$120/year`, or `$199 perpetual`, single user. Includes LLM-assisted detection, model setup and updates, and the full review workflow. Perpetual licenses include 12 months of updates; optional maintenance renewal, roughly `$49/year`, keeps updates active after that. Offer subscription and perpetual options side by side at checkout.
- Firm: `$25/user/month` or roughly `$240/user/year`. Includes centralized license management, admin console, priority support, metadata-only audit log, and a commercial agreement suitable for procurement.
- Enterprise: custom pricing. Includes SSO, on-prem license server, custom audit-log retention policies, security questionnaires, deployment support, and procurement/security review.

Firm and Enterprise should be possible in the architecture but absent from Phase 1 UI. Do not let multi-user administration, audit logs, procurement workflows, or SSO leak into the first single-user product.

### Trial Mechanics

- Trial lasts 14 days and unlocks Pro features.
- No credit card is required to start a trial.
- Trial expiry transitions the app to Free tier rather than locking the user out.
- Trial expiry must call into the same license state transition system as paid license changes so the fallback behavior is tested and reliable.

### What We Will Not Do

- No usage-based pricing.
- No analytics or telemetry.
- No ads.
- No credit card required for trial.
- No Free tier degradation over time.
- No app-store distribution initially.

Licensing should be privacy-preserving. Activation may contact a license server with license metadata and device/app identifiers, but never user content or pseudonymization artifacts. The app should continue to offer deterministic/manual functionality when offline or unlicensed.

Keep the open-sourceable boundary explicit: the license activation flow, license server URL, signing keys, entitlement checks, paid feature gates, model download manager, model runtime, auto-update mechanism, platform-specific installer logic, and frontend UI all live outside `pseudo-core`. Publishing `pseudo-core` later must not reveal license-validation internals or proprietary product infrastructure.

The app should avoid asking for license activation before the user has interacted with the core workflow. Activation can unlock the LLM-assisted path, longer text limits, or commercial support, but the initial product experience should remain useful, local, and inspectable.

## 13. Privacy and Security Requirements

- All analysis must run locally.
- No analytics or telemetry in MVP.
- Do not persist pasted text unless the user explicitly saves a project later.
- Do not log user text in Rust or the frontend console.
- Never send user content or pseudonymization artifacts to any network service. This includes pasted text, imported document content, saved project content, manually marked text, findings, replacement maps, and pseudonymized output.
- License activation, model download, update checks, and enterprise/admin sync may send only non-content metadata required for those operations.
- `pseudo-core` must make no network calls of any kind. It must not include update checks, license checks, model downloads, crash reporting, telemetry, HTTP clients, socket clients, or model runtime orchestration.
- All network activity must live in the outer app/runtime crates and be auditable at that boundary.
- Firm and Enterprise audit logging records metadata only: timestamp, local user identifier, analysis duration, finding count, and finding types as aggregate counts. It must never record source text, finding surface forms, replacement maps, or pseudonymized output.
- Audit log storage is local to each user's machine by default. Admin-console sync, when enabled, should sync aggregate metadata only and never content.
- Audit logging is configurable by the firm admin and can be fully disabled. The product must continue working without it.
- The local model runs in-process. No localhost ports are opened.
- No external network connections are made during analysis.
- Clear in-memory state when the user clicks `Clear`.
- Document that the app assists pseudonymization but does not guarantee formal anonymization for every regulatory or professional standard.

Audit event schema:

```ts
type AnalysisAuditEvent = {
  id: string;
  occurredAt: string;
  localUserId: string;
  licenseTier: "FIRM" | "ENTERPRISE";
  analysisDurationMs: number;
  findingCount: number;
  findingTypeCounts: Record<SensitiveType, number>;
  engineMode: "BASIC" | "LOCAL_MODEL";
};
```

Explicitly not recorded:

- source text
- exact finding text
- replacement map
- pseudonymized result
- file names or document titles unless a future admin setting explicitly enables them

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
      core/
        analysisTypes.ts
        replacement.ts
        chunkTextForAnalysis.ts
        sampleAnalysis.ts
      app/
        tauriApi.ts
        licenseStatus.ts
        modelStatus.ts
    admin/
      AdminApp.tsx
      components/
        LicenseSeatTable.tsx
        AuditLogSummary.tsx
      lib/
        adminApi.ts
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
        license.rs
        audit_log.rs
        model_download.rs
        model_runtime.rs
  deny.toml
  docs/
    implementation-plan.md
```

This repository currently contains the plan at the root. Once the app scaffold exists, move or copy this document to `docs/implementation-plan.md`.

`pseudo-core` should be independently buildable and testable. The Tauri app crate and `pseudo-cli` should depend on it as consumers rather than duplicating deterministic logic. The CLI should be usable in CI for end-to-end deterministic analysis tests without launching the desktop UI.

`src-tauri/app/src/model_runtime.rs` owns the `llama-cpp-2` integration, model lifecycle, GBNF grammar definition, and inference loop. If this code grows beyond roughly 1000 LOC, move it into a dedicated closed `src-tauri/pseudo-runtime` crate so `app/` stays focused on commands and orchestration. Do not move inference code into `pseudo-core`.

The admin console path is reserved for Firm and Enterprise tiers, but it should not be built in Phase 1. It can begin as a separate frontend bundle under `src/admin/` and later move to a web-based dashboard that talks to an on-prem license server for Enterprise deployments.

## 15. Testing Plan

### Unit Tests

Frontend:

- analysis state transitions: `SAMPLE_READY`, `EMPTY`, `DIRTY_NEEDS_ANALYSIS`, `ANALYZING`, `ANALYZED_READY`, and `ERROR`
- editing source text after analysis dims stale highlights, freezes stale preview, disables replacement editing/manual marking/copy, and shows `Review paused · analyze again`
- text chunking preserves offsets
- grouping repeated findings
- suggested replacement numbering
- replacement memory preserves user edits, enabled state, and numbering across re-analysis within the same session
- newly inserted earlier findings do not renumber existing remembered replacements
- exact-normalized grouping does not merge aliases such as `Jane`, `Jane Doe`, and `Ms. Doe` in Phase 1
- optimistic pseudonymized preview generation after analysis
- replacement edits and toggles update optimistic preview synchronously without backend calls
- final copy reconciles optimistic preview with backend-confirmed output
- stale analysis/manual/recompute responses are ignored by `requestId`
- disabled replacements are skipped
- readiness summary rules for `Ready to copy`, `Manual review recommended`, disabled groups, empty replacements, low-confidence findings, `OTHER_SENSITIVE`, and dirty/error states
- selection-to-manual-finding UI behavior
- `src/lib/core` utilities remain pure and do not import Tauri/app glue

Rust:

- `pseudo-core` regex detection for structured sensitive info
- `pseudo-core` deterministic rules support English and German labels/month names where relevant
- `pseudo-core` phone detection accepts validated international/area-code numbers through `phonenumber` and rejects bare local numbers such as `5551234`
- `pseudo-core` date detection accepts month-name and ISO dates, accepts unambiguous numeric dates, and rejects ambiguous numeric dates such as `12.03.2025` and `03/12/2025`
- `pseudo-core` ID detection accepts contextual uppercase/digit hyphenated IDs and rejects common false positives such as `COVID-19` and `GDPR-compliant`
- `pseudo-core` has no Tauri, networking, license, model-download, model-runtime, telemetry, or update dependencies
- `pseudo-core` dependency licenses pass `cargo deny`
- Tauri app crate model output validation
- Tauri app crate model download status and checksum/manifest validation
- exact surface-form matching from LLM output to source ranges
- overlap resolution
- overlap resolver property tests: output is sorted, non-overlapping, every winner exactly matches its source slice, resolver is deterministic under input permutation, and adding a lower-priority overlapping candidate cannot remove a higher-priority winner
- overlap resolver fixtures for deterministic-vs-deterministic conflicts, email-vs-URL/domain overlap, date-inside-ID overlap, LLM partial overlap, same-span type disagreement, manual override, and longer-contained-span wins
- canonical grouping and replacement application from end to start
- manual finding validation for selected ranges
- stale text hash rejection for apply/recompute/copy paths
- license status and activation state handling without user text
- model runtime status handling for loaded/unloaded/backend/quantization/load time/resident memory
- `app::model_runtime` or `pseudo-runtime` loads a fixture GGUF model and returns valid JSON for a known chunk
- GBNF grammar produces only schema-conformant JSON across a representative test set
- model idle unload releases memory in a smoke test
- first LLM-assisted analysis cold-load completes under a documented reference budget, for example 5 seconds on an M1 MacBook Air
- model load failure and inference failure fall back to deterministic/manual analysis without clearing user work
- `pseudo-cli analyze < input.txt > output.json` returns deterministic findings and replacement groups without launching the UI

### Integration Tests

- fresh install, no model, no license: open app and immediately see an already-analyzed confidential-work sample with highlights, grouped replacements, before/after preview, readiness summary, locality proof, and enabled `Copy result`
- first launch uses bundled sample fixtures and does not call `analyze_text` before first paint
- bundled sample fixture text hash matches the sample text and pseudonymized output
- click `Try your own text`, clear the sample, focus the editor, and show `Paste confidential text. No text or results are sent.`
- paste sample text
- verify pasted or edited text enters `DIRTY_NEEDS_ANALYSIS` without auto-analysis
- run deterministic-only analysis on user-provided text without model setup
- run deterministic-only analysis
- user-initiated deterministic analysis of the sample fixture completes under 500 ms on normal desktop hardware
- edit analyzed source text and verify stale highlights are dimmed, copy is disabled, and `Analyze` is primary
- show highlights
- mark selected text as sensitive
- edit replacement
- verify pseudonymized preview updates automatically after replacement edits while in `ANALYZED_READY`
- verify replacement edits do not call backend recomputation until copy/reset/manual-marking
- verify older in-flight analysis responses are ignored when a newer request has started
- verify readiness summary reflects enabled, disabled, ignored, and review-needed findings
- copy backend-confirmed final result and show `Pseudonymized text copied · no text or results sent`
- optional model download completes and enables local LLM analysis
- expired/unlicensed state falls back to deterministic/manual functionality

### First-Run Sample and Manual Test Text

```text
Please prepare the draft settlement note for Jane Doe, the senior claims manager at ACME Health.
She emailed john.smith@example.com on 12 March 2025 about patient ID PT-44921, and asked that we confirm her Zurich address before Friday's call at +41 44 123 45 67.
The open invoice is INV-2025-991; please do not include the internal link https://acme.example/matters/44921 in anything we send to opposing counsel.
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
- https://acme.example/matters/44921 -> `[URL_1]`

## 16. Build Phases

### Phase 1: First-Impression Deterministic MVP

- Create Tauri + React + TypeScript app
- Create `pseudo-core` as a pure Rust library and keep deterministic detectors, chunking, grouping, overlap resolution, replacement, manual-finding validation, and shared types inside it
- Create `pseudo-cli` as a thin wrapper around `pseudo-core` for UI-free deterministic analysis and integration tests
- Add `cargo deny` with permissive-license policy for `pseudo-core`
- Build a polished first-launch before/after demo layout with a compact trust/status strip: `No text or results are sent`, `Nothing saved`, `Basic detection active`, and `Clear`
- Add preloaded confidential-work sample text that opens already analyzed using bundled static fixtures for text, findings, replacement groups, replacement memory, pseudonymized output, and text hash
- Make `Try your own text` the primary first-run action; it clears the sample, focuses the editor, and shows `Paste confidential text. No text or results are sent.`
- Hide license activation, account creation, settings, model download, model status, upgrade banners, and model setup from the primary first-run path
- Implement paste/edit text area
- Implement deterministic detectors for email, phone, URL, dates, and ID-like values
- Include English and German deterministic fixtures for date month names, ID context labels, and phone context
- Implement canonical Rust grouping and range-based replacement application
- Implement session-local replacement memory so user edits and stable numbering survive re-analysis
- Implement highlighting and replacement panel
- Implement immediate range-based pseudonymized preview
- Keep source highlights, replacement rows, and pseudonymized result synchronized
- Add manual "mark selected text as sensitive"
- Add deterministic readiness summary before copy, including findings count, enabled replacements, disabled/ignored counts, and items needing review
- Add honest first-launch locality proof, such as `Demo ready locally · no text or results sent`, and show measured timing only after user-initiated analysis
- Add copy toast `Pseudonymized text copied · no text or results sent`, clear action, and polished empty/no-findings/error states
- Add unit tests for replacement logic
- Add an automated first-run golden-path test for fresh install, no model, no license, already-analyzed sample, before/after preview, review, locality proof, and copy

Deliverable: useful, trustworthy app without LLM dependency that demonstrates value within seconds.

### Phase 2: Review Quality and Workflow Confidence

- Add click-to-focus between highlight and replacement row
- Add type filters
- Add confidence display or review badges
- Add "ignore finding" action
- Add explicit "reset to suggested replacements" action
- Add clear disabled/ignored finding visibility in the readiness summary
- Keep these controls progressively disclosed through `More` or on later analyses so the first screen remains focused on see, understand, copy
- Improve chunk boundary selection

Deliverable: practical review workflow that feels controlled, inspectable, and ready for real client text.

Phase 3 entry decisions to lock before implementation:

- Quantization default: start with Q4_K_M, but run an in-house eval against realistic confidential drafts and compare with Q8_0 before public release.
- macOS backend: use Metal for Apple Silicon unless CI or packaging evidence proves the complexity is not yet worth it.
- Qwen3 mode: use non-thinking mode for grammar-constrained span extraction; do not spend latency on chain-of-thought tokens when the required output is structured JSON.

### Phase 3: Local LLM via llama.cpp (In-Process)

- Add `llama-cpp-2` to the Tauri app crate or a dedicated closed `pseudo-runtime` crate. Confirm `cargo deny` allows the MIT OR Apache-2.0 dependency.
- Implement the model download manager for `ggml-org/Qwen3-1.7B-GGUF:Q4_K_M` by default, with progress, resumable download, checksum/manifest validation, and preserved upstream `LICENSE`/`NOTICE` files. Re-check the Qwen namespace before implementation and prefer an official Qwen Q4_K_M file if one exists then.
- Implement `model_runtime`: model load, idle unload, single-session inference, backend reporting, memory reporting, and a GBNF grammar that constrains output to the `{findings: [...]}` schema.
- Run Qwen3 in non-thinking mode for span extraction so the model emits only the structured result the grammar allows.
- Add chunk batching: send N chunks per inference call, start with N=4, tune empirically, parse the structured JSON output, and pass `(chunk_index, surface_form, type, confidence)` tuples to `pseudo-core` for offset resolution.
- Merge LLM findings with deterministic findings using the existing overlap-resolution rules in `pseudo-core`.
- Add model status UI and post-analysis upgrade prompt only after the user has run deterministic analysis on their own text.
- Add model-assisted alias/co-reference suggestions as reviewable proposals, not automatic merges.
- Backend selection per platform: Metal on macOS, CPU on Windows/Linux for the default installer. CUDA and Vulkan builds are Phase 4 packaging decisions, not default v1 requirements.

Deliverable: local LLM-assisted detection running in-process with no Python dependency, guided model setup, and a single-binary installer story.

### Phase 3.5: Open-Source Readiness Review

Do this after Phase 3 and after the first paying customers validate the product shape. The goal is to make `pseudo-core` publishable without dragging the full desktop product into an open-source support burden too early.

- Decide whether to publish `pseudo-core` and `pseudo-cli` as a separate public repository or public package inside the main repository
- Use Apache-2.0 unless there is a specific reason to choose another permissive license
- Add `SECURITY.md` with a vulnerability disclosure policy and contact
- Add `CONTRIBUTING.md` with a DCO sign-off requirement instead of a CLA
- Add public CI for format, tests, `cargo deny`, and CLI smoke tests
- Audit git history and release artifacts for proprietary remnants, license-server details, signing keys, real client fixtures, model credentials, and private infrastructure URLs
- Keep the model download manager, license client, model runtime packaging, platform installers, auto-update mechanism, frontend UI, and real-client-derived fixtures closed
- Publish only when there is enough customer or buyer value to justify issue triage, security disclosures, contribution review, and governance overhead

Deliverable: an auditable deterministic engine and CLI that security teams can inspect and run offline, while the commercial desktop product remains closed.

### Phase 4: Packaging

- Statically link or bundle the llama.cpp shared library inside the Tauri app binary. Verify on each target platform, macOS arm64, macOS x86_64, Windows x86_64, and Linux x86_64, that the binary runs without external runtime dependencies beyond standard OS libraries.
- Keep installer small by excluding the model from the app bundle
- Distribute through direct download from the product website with signed installers for macOS, Windows, and Linux
- Do not use app stores in v1 because of the revenue cut, model/runtime packaging constraints, and reduced ability to communicate directly with buyers
- Decide CUDA Windows build distribution: ship as a separate optional download once a customer asks; do not include CUDA in the default installer because of size
- Ship `third-party-notices.md` including the Apache-2.0 license for Qwen3-1.7B, MIT license for llama.cpp, and MIT OR Apache-2.0 license notice for `llama-cpp-2`
- Add model download/update/delete settings
- Add model path settings screen
- Add privacy-preserving trial/license activation for Free, Trial, Pro subscription, Pro perpetual, Firm, and Enterprise states
- Add subscription and perpetual license infrastructure, including perpetual maintenance windows and update eligibility
- Add trial-to-Free fallback logic so expiry never locks the user out
- Add metadata-only analysis event recording behind Firm/Enterprise license gates
- Add platform-specific packaging notes for macOS, Windows, and Linux
- Validate offline startup
- Validate no user text appears in logs

Deliverable: installable cross-platform desktop app suitable for confidential-work professionals and the initial lawyer wedge.

### Phase 5: Firm and Advanced Features

- Export pseudonymization map
- Import/export replacement presets
- Project/session save with explicit user consent
- Support structured documents
- Admin console
- Multi-user license management
- Audit log viewer for firm admins, using metadata-only events
- Priority support tooling
- Optional stronger local NER model or fine-tuned classifier
- Optional larger or fine-tuned local model variant after real-user evaluation

Deliverable: Firm-ready product with centralized license management and auditability without content collection.

### Phase 6: Enterprise

- SSO integration
- On-prem license server
- Deployment scripts and IT packaging guidance
- Custom audit-log retention controls
- Security questionnaire support
- Hardened admin controls for large deployments

Deliverable: Enterprise deployment path for organizations that need procurement, identity, and on-prem controls.

## 17. Key Risks and Mitigations

### LLM Output Is Not Reliably Structured

Mitigation:

- Keep deterministic detectors
- Use llama.cpp GBNF grammar to constrain output to the expected JSON schema
- Validate all model output
- Discard invalid spans
- Show warnings rather than failing the whole analysis

### Pseudonymization May Miss Sensitive Data

Mitigation:

- Present the app as assisted review
- Add manual marking quickly
- Prefer conservative highlighting
- Provide visible warning when model is unavailable

### Packaging the Local Model Runtime

Mitigation:

- Use `llama-cpp-2`, Rust bindings to llama.cpp, for in-process inference. No Python runtime is bundled.
- Keep the app installer small and download the model after install
- Static-link or vendor llama.cpp so the app binary has no external runtime dependencies beyond standard OS libraries
- Treat GPU backends, especially CUDA and Vulkan, as separate optional installer variants rather than bundling them into the default download

### Model Download or License Flow Hurts Trust

Mitigation:

- Require explicit user approval before model download
- Show deterministic/manual value on the user's own text before asking the user to download a model or activate a license
- Do not show model download, activation, account creation, or settings as primary actions on the first screen
- Show model size, destination, and local-only analysis promise before setup
- Never send user content or pseudonymization artifacts during setup, activation, update checks, model download, or admin sync
- Keep deterministic/manual functionality available without activation
- Qwen3-1.7B license and hosted-download rights are resolved for the current model choice under Apache-2.0; preserve upstream license files and re-verify before changing models

### Free Tier Cannibalizes Paid

Mitigation:

- Keep Free genuinely useful rather than degrading it artificially
- Make paid genuinely more thorough with local LLM detection for names, organizations, roles, and context-sensitive spans that regex cannot reliably catch
- Communicate the difference honestly: Free helps quickly, Pro catches more subtle identifying context
- For the initial buyer, a single missed client name can be high consequence, so the paid upgrade has real value without crippling Free

### Pricing Is Wrong on Launch

Mitigation:

- Launch with the suggested numbers and expect to revisit them
- Instrument nothing, because telemetry conflicts with the privacy positioning
- Talk directly to the first dozen paying customers and adjust within the first 90 days
- Do not run pricing A/B tests early; there is not enough statistical power, and the infrastructure creates privacy and trust costs

### Core Becomes Hard to Open-Source Later

Mitigation:

- Keep deterministic logic in `pseudo-core` from the first implementation pass
- Keep networking, licensing, model downloads, model runtime orchestration, telemetry, updates, and packaging out of `pseudo-core`
- Enforce permissive dependency licenses with `cargo deny`
- Use `pseudo-cli` and `pseudo-core` tests as the compatibility contract
- Avoid real client fixtures and proprietary app details in core tests

### Replacing Text Naively Can Corrupt Output

Mitigation:

- Apply only validated character ranges
- Sort ranges descending
- Use Rust as the canonical replacement engine
- Confirm the final copied/exported result through the backend
- Keep tests around overlapping and repeated spans

## 18. Immediate Next Steps

1. Scaffold Tauri + React + TypeScript project.
2. Create a Rust workspace with `pseudo-core`, `pseudo-cli`, and the Tauri app crate.
3. Add `cargo deny` with permissive-license policy for `pseudo-core`.
4. Build the first-run shell: pre-analyzed confidential-work sample, trust/status strip, before/after layout, replacement panel, enabled `Copy result`, `Try your own text`, and clear action.
5. Implement shared TypeScript types in `src/lib/core` plus bundled sample findings/mock analysis results to perfect highlights, replacement rows, readiness summary, locality proof, copy confirmation, and empty/no-findings/error states.
6. Implement `pseudo-core` grouping and range-based replacement utilities.
7. Add deterministic detectors in `pseudo-core` and wire them through a Tauri command.
8. Add `pseudo-cli analyze < input.txt > output.json` for deterministic engine smoke tests.
9. Replace mock analysis with deterministic backend results and keep source highlights, replacement rows, and pseudonymized preview synchronized.
10. Add manual selected-text marking.
11. Add the first-run golden-path test and unit tests for grouping, overlap resolution, manual findings, replacement, dependency license policy, and readiness summary state.
12. Add review workflow controls: click-to-focus, ignore finding, filters, and reset suggested replacements.
13. Add optional model setup UI state with placeholder download/status behavior, visible only after the user has seen a successful analysis on their own text.
14. Add `llama-cpp-2` integration in a new `model_runtime` module inside the Tauri app crate, or a closed `pseudo-runtime` crate, after the deterministic review workflow feels solid. Use `ggml-org/Qwen3-1.7B-GGUF:Q4_K_M` as the default model target unless an official Qwen-namespace Q4_K_M file exists before implementation.
15. Write a GBNF grammar that constrains llama.cpp output to the `{findings: [...]}` schema. Add a unit test that runs the grammar against a fixture model and asserts the output parses.
16. Write the licensing state machine in Rust before adding paid features: `Free`, `Trial` with expiry timestamp, `ProSubscription` with renewal date, `ProPerpetual` with maintenance-active flag and update-eligible-until date, `Firm`, and `Enterprise`.
17. Draft the audit log schema and explicit list of fields recorded and not recorded as a privacy commitment document that ships with the product.
