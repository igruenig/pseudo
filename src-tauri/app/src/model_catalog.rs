use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub name: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub expected_bytes: u64,
}

const SMOLLM3: ModelSpec = ModelSpec {
    name: "SmolLM3-3B Q4_K_M",
    file_name: "smollm3-3b-q4_k_m.gguf",
    url: "https://huggingface.co/ggml-org/SmolLM3-3B-GGUF/resolve/main/SmolLM3-Q4_K_M.gguf",
    expected_bytes: 1_915_305_312,
};

const QWEN3: ModelSpec = ModelSpec {
    name: "Qwen3-1.7B Q4_K_M",
    file_name: "qwen3-1.7b-q4_k_m.gguf",
    url: "https://huggingface.co/ggml-org/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q4_K_M.gguf",
    expected_bytes: 1_282_439_264,
};

pub fn active_model() -> ModelSpec {
    match std::env::var("PSEUDO_MODEL")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "qwen" | "qwen3" | "qwen3-1.7b" => QWEN3,
        "smol" | "smollm" | "smollm3" | "smollm3-3b" | "" => SMOLLM3,
        _ => SMOLLM3,
    }
}

pub fn active_model_path() -> PathBuf {
    if let Ok(path) = std::env::var("PSEUDO_MODEL_PATH") {
        return PathBuf::from(path);
    }
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("pseudo")
        .join("models")
        .join(active_model().file_name)
}
