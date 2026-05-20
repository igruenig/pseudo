#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChunk {
    pub chunk_index: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

pub fn chunk_text_for_analysis(text: &str, max_chars: usize) -> Vec<TextChunk> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = next_boundary(text, start, max_chars);
        chunks.push(TextChunk {
            chunk_index: chunks.len(),
            start,
            end,
            text: text[start..end].to_string(),
        });
        start = end;
        while start < text.len() && text[start..].starts_with('\n') {
            start += 1;
        }
    }
    chunks
}

fn next_boundary(text: &str, start: usize, max_chars: usize) -> usize {
    let hard_end = text
        .char_indices()
        .map(|(idx, _)| idx)
        .filter(|idx| *idx > start)
        .nth(max_chars)
        .unwrap_or(text.len());
    if hard_end == text.len() {
        return text.len();
    }

    let window = &text[start..hard_end];
    for pattern in ["\n\n", ". ", "? ", "! "] {
        if let Some(pos) = window.rfind(pattern) {
            return start + pos + pattern.len();
        }
    }
    if let Some(pos) = window.rfind(char::is_whitespace) {
        return start + pos + 1;
    }
    hard_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_source_slices() {
        let text = "One sentence. Two sentence.\n\nThree.";
        let chunks = chunk_text_for_analysis(text, 18);
        for chunk in chunks {
            assert_eq!(&text[chunk.start..chunk.end], chunk.text);
        }
    }
}
