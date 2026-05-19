export type SensitiveType =
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

export type FindingSource = "DETERMINISTIC" | "LLM" | "MANUAL";

export type Finding = {
  id: string;
  type: SensitiveType;
  // Offsets are UTF-8 byte offsets from Rust. Convert before slicing JavaScript strings.
  start: number;
  end: number;
  text: string;
  source: FindingSource;
  confidence?: number;
  needsReview?: boolean;
};

export type ReplacementGroup = {
  id: string;
  type: SensitiveType;
  original: string;
  normalizedOriginal: string;
  replacement: string;
  findingIds: string[];
  enabled: boolean;
};

export type AnalysisResult = {
  requestId: string;
  sourceTextHash: string;
  findings: Finding[];
  groups: ReplacementGroup[];
  pseudonymizedText: string;
  warnings: string[];
};

export type ModelStatus = {
  loaded: boolean;
  modelPath?: string;
  quantization?: string;
  backend: "metal" | "cpu";
  loadMs?: number;
  residentMemoryMb?: number;
};

export type ModelDownloadStatus = {
  state: "not_started" | "downloading" | "complete" | "error" | "cancelled";
  modelName: string;
  destinationPath: string;
  bytesDownloaded: number;
  totalBytes?: number;
  error?: string;
};

export type AnalysisStateName =
  | "EMPTY"
  | "DIRTY_NEEDS_ANALYSIS"
  | "ANALYZING"
  | "ANALYZED_READY"
  | "ERROR";
