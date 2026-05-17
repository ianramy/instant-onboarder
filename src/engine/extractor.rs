//! Zero-token local code outline extractor.
//!
//! This module provides an ultra-lightweight parsing engine capable of generating
//! structural summaries of Rust source code without needing an external AI model.
//! It operates entirely offline, making it extremely fast. It works by scanning the file
//! line-by-line to extract module documentation, item docstrings, and major struct or
//! function definitions.

/// Extracts a structural Markdown outline directly from raw source code.
///
/// This function executes a rapid, single-pass lexical analysis of the provided code string.
/// It does not build a full Abstract Syntax Tree (AST); instead, it intelligently identifies
/// key structural markers (like `pub struct` or `fn`) and automatically bundles them with
/// any adjacent documentation it discovers immediately above them.
///
/// Features & Comment Handling
/// - Module-Level Documentation: It instantly captures `//!` comments and pins them
///   to the very top of the generated Markdown outline. These module docs are treated as
///   global context and are safely preserved regardless of the surrounding code.
/// - Item-Level Documentation: It buffers standard `///` docstrings.
///   When a major symbol (like an enum or function) is encountered, the buffered
///   documentation is visually attached to that symbol as a formatted blockquote.
/// - Contextual Integrity: If the parser encounters standard executing code, variable
///   bindings, or import statements before reaching a symbol, it intelligently flushes
///   the doc buffer. This prevents completely unrelated, older comments from being
///   erroneously attached to the next struct down the file.
pub fn extract_outline(content: &str) -> String {
    let mut outline = String::from("### 🔍 Premium Architectural Outline\n\n");
    outline.push_str("*AI analysis disabled. Instantly extracted from source code:*\n\n");

    let mut doc_buffer: Vec<String> = Vec::new();
    let mut found_symbols = false;
    let mut found_module_docs = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // 1. Handle module-level documentation (e.g., `//!`)
        // These belong at the top of the file, so we inject them straight into the outline
        // rather than buffering them.
        if trimmed.starts_with("//!") {
            let clean_doc = trimmed.trim_start_matches("//!").trim();
            if !clean_doc.is_empty() {
                outline.push_str(&format!("> *{}*\n", clean_doc));
            }
            found_module_docs = true;
            continue;
        }

        // 2. Handle item-level docstrings (`///`) and standard comments (`//`)
        // We buffer these and wait for a structural block (like a struct or fn) to attach them to.
        if trimmed.starts_with("//") {
            // Strip all leading slashes and any remaining formatting artifacts
            let clean_doc = trimmed
                .trim_start_matches('/')
                .trim_start_matches('!')
                .trim();
            if !clean_doc.is_empty() {
                doc_buffer.push(clean_doc.to_string());
            }
            continue;
        }

        // 3. Identify major structural blocks
        if trimmed.starts_with("pub struct")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub fn")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("pub enum")
            || trimmed.starts_with("enum ")
        {
            let clean_line = trimmed.trim_end_matches('{').trim();

            // Add the symbol name bolded and as inline code
            outline.push_str(&format!("`{}`\n", clean_line));

            // If we found docstrings or comments directly above it, add them as a premium blockquote
            if !doc_buffer.is_empty() {
                for doc in &doc_buffer {
                    outline.push_str(&format!("> *{}*\n", doc));
                }
                outline.push('\n');
            } else {
                outline.push_str("> *(No documentation provided)*\n\n");
            }

            // Reset buffer for the next symbol
            doc_buffer.clear();
            found_symbols = true;
        } else if !trimmed.is_empty() {
            // If it's normal executing code or an import, clear the buffer so we
            // don't accidentally attach old disconnected comments to new structs
            doc_buffer.clear();
        }
    }

    // Add a spacer if we found module docs but haven't written symbols yet
    if found_module_docs && found_symbols {
        outline = outline.replace("> *\n`", "> *\n\n`");
    }

    if !found_symbols && !found_module_docs {
        return "### 🔍 Premium Architectural Outline\n\nNo major structs, functions, or documentation found in this file.".to_string();
    }

    outline
}
