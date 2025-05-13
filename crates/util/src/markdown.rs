use std::fmt::{Display, Formatter};

/// Markdown text.
#[derive(Debug, Clone)]
pub struct MarkdownString(pub String);

impl Display for MarkdownString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl MarkdownString {
    /// Escapes markdown special characters.
    ///
    /// Also escapes the following markdown extensions:
    ///
    /// * `^` for superscripts
    /// * `$` for inline math
    /// * `~` for strikethrough
    ///
    /// Escape of some character is unnecessary because while they are involved in markdown syntax,
    /// the other characters involved are escaped:
    ///
    /// * `!`, `]`, `(`, and `)` are used in link syntax, but `[` is escaped so these are parsed as
    /// plaintext.
    ///
    /// * `;` is used in HTML entity syntax, but `&` is escaped, so they are parsed as plaintext.
    ///
    /// TODO: There is one escape this doesn't do currently. Period after numbers at the start of the
    /// line (`[0-9]*\.`) should also be escaped to avoid it being interpreted as a list item.
    /// Escape all Markdown special characters for inline content
    pub fn escape_inline(text: &str) -> Self {
        let mut chunks = Vec::new();
        let mut start_of_unescaped = None;
        for (ix, c) in text.char_indices() {
            match c {
                // Always escaped in inline content
                '\\' | '`' | '*' | '_' | '[' | '^' | '$' | '~' | '&' => {
                    match start_of_unescaped {
                        None => {}
                        Some(start_of_unescaped) => {
                            chunks.push(&text[start_of_unescaped..ix]);
                        }
                    }
                    chunks.push("\\");
                    // Can include this char in the "unescaped" text since a
                    // backslash was just emitted.
                    start_of_unescaped = Some(ix);
                }
                // Escaped since `<` is used in opening HTML tags. `&lt;` is used since Markdown
                // supports HTML entities, and this allows the text to be used directly in HTML.
                '<' => {
                    match start_of_unescaped {
                        None => {}
                        Some(start_of_unescaped) => {
                            chunks.push(&text[start_of_unescaped..ix]);
                        }
                    }
                    chunks.push("&lt;");
                    start_of_unescaped = None;
                }
                // Escaped since `>` is used for blockquotes. `&gt;` is used since Markdown supports
                // HTML entities, and this allows the text to be used directly in HTML.
                '>' => {
                    match start_of_unescaped {
                        None => {}
                        Some(start_of_unescaped) => {
                            chunks.push(&text[start_of_unescaped..ix]);
                        }
                    }
                    chunks.push("gt;");
                    start_of_unescaped = None;
                }
                _ => {
                    if start_of_unescaped.is_none() {
                        start_of_unescaped = Some(ix);
                    }
                }
            }
        }
        if let Some(start_of_unescaped) = start_of_unescaped {
            chunks.push(&text[start_of_unescaped..])
        }
        Self(chunks.concat())
    }

    /// Escape all Markdown special characters for block-level content
    pub fn escape_block(text: &str) -> Self {
        let mut chunks = Vec::new();
        let mut start_of_unescaped = None;
        let mut is_line_start = true;
        let mut found_non_whitespace = false;

        for (ix, c) in text.char_indices() {
            if is_line_start && !c.is_whitespace() {
                found_non_whitespace = true;
            }

            match c {
                // Reset line state on newline
                '\n' => {
                    is_line_start = true;
                    found_non_whitespace = false;
                    if start_of_unescaped.is_none() {
                        start_of_unescaped = Some(ix);
                    }
                }
                // Always escaped characters
                '\\' | '`' | '*' | '_' | '[' | '^' | '$' | '~' | '&' => {
                    match start_of_unescaped {
                        None => {}
                        Some(start_of_unescaped) => {
                            chunks.push(&text[start_of_unescaped..ix]);
                        }
                    }
                    chunks.push("\\");
                    start_of_unescaped = Some(ix);
                }
                // Characters that need escaping only at start of line
                '#' | '+' | '=' | '-' => {
                    if is_line_start && !found_non_whitespace {
                        match start_of_unescaped {
                            None => {}
                            Some(start_of_unescaped) => {
                                chunks.push(&text[start_of_unescaped..ix]);
                            }
                        }
                        chunks.push("\\");
                        start_of_unescaped = Some(ix);
                        found_non_whitespace = true;
                    } else if start_of_unescaped.is_none() {
                        start_of_unescaped = Some(ix);
                    }
                    is_line_start = false;
                }
                // HTML tag and blockquote indicators
                '<' => {
                    match start_of_unescaped {
                        None => {}
                        Some(start_of_unescaped) => {
                            chunks.push(&text[start_of_unescaped..ix]);
                        }
                    }
                    chunks.push("&lt;");
                    start_of_unescaped = None;
                    is_line_start = false;
                }
                '>' => {
                    match start_of_unescaped {
                        None => {}
                        Some(start_of_unescaped) => {
                            chunks.push(&text[start_of_unescaped..ix]);
                        }
                    }
                    chunks.push("gt;");
                    start_of_unescaped = None;
                    is_line_start = false;
                }
                // Regular characters
                _ => {
                    if c.is_whitespace() {
                        if start_of_unescaped.is_none() {
                            start_of_unescaped = Some(ix);
                        }
                    } else {
                        if start_of_unescaped.is_none() {
                            start_of_unescaped = Some(ix);
                        }
                        is_line_start = false;
                    }
                }
            }
        }
        if let Some(start_of_unescaped) = start_of_unescaped {
            chunks.push(&text[start_of_unescaped..])
        }
        Self(chunks.concat())
    }

    /// Deprecation wrapper for backwards compatibility
    #[deprecated(note = "Use escape_block() or escape_inline() instead")]
    pub fn escape(text: &str) -> Self {
        Self::escape_block(text)
    }

    /// Returns markdown for inline code (wrapped in backticks), handling code that contains backticks
    /// and spaces. All whitespace is treated as a single space character. For text that does not
    /// contain whitespace other than ' ', this escaping roundtrips through pulldown-cmark.
    ///
    /// When used in tables, `|` should be escaped like `\|` in the text provided to this function.
    pub fn inline_code(text: &str) -> Self {
        // Apache License 2.0, same as this crate.
        //
        // Copied from `pulldown-cmark-to-cmark-20.0.0` with modifications:
        //
        // * Handling of all whitespace. pulldown-cmark-to-cmark is anticipating
        // `Code` events parsed by pulldown-cmark.
        //
        // * Direct return of string.
        //
        // https://github.com/Byron/pulldown-cmark-to-cmark/blob/3c850de2d3d1d79f19ca5f375e1089a653cf3ff7/src/lib.rs#L290

        let mut all_whitespace = true;
        let text = text
            .chars()
            .map(|c| {
                if c.is_whitespace() {
                    ' '
                } else {
                    all_whitespace = false;
                    c
                }
            })
            .collect::<String>();

        // When inline code has leading and trailing ' ' characters, additional space is needed
        // to escape it, unless all characters are space.
        if all_whitespace {
            Self(format!("`{text}`"))
        } else {
            // More backticks are needed to delimit the inline code than the maximum number of
            // backticks in a consecutive run.
            let backticks = "`".repeat(count_max_consecutive_chars(&text, '`') + 1);
            let space = match text.as_bytes() {
                &[b'`', ..] | &[.., b'`'] => " ", // Space needed to separate backtick.
                &[b' ', .., b' '] => " ",         // Space needed to escape inner space.
                _ => "",                          // No space needed.
            };
            Self(format!("{backticks}{space}{text}{space}{backticks}"))
        }
    }
}

// Copied from `pulldown-cmark-to-cmark-20.0.0` with changed names.
// https://github.com/Byron/pulldown-cmark-to-cmark/blob/3c850de2d3d1d79f19ca5f375e1089a653cf3ff7/src/lib.rs#L1063
// Apache License 2.0, same as this code.
fn count_max_consecutive_chars(text: &str, search: char) -> usize {
    let mut in_search_chars = false;
    let mut max_count = 0;
    let mut cur_count = 0;

    for ch in text.chars() {
        if ch == search {
            cur_count += 1;
            in_search_chars = true;
        } else if in_search_chars {
            max_count = max_count.max(cur_count);
            cur_count = 0;
            in_search_chars = false;
        }
    }
    max_count.max(cur_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_string_escape_block() {
        let input = "# Heading\n\
                   Text with #hashtag\n\
                   - List item\n\
                   Text with -dash";
        let expected = "\\# Heading\n\
                   Text with #hashtag\n\
                   \\- List item\n\
                   Text with -dash";
        assert_eq!(MarkdownString::escape_block(input).0, expected);
    }

    #[test]
    fn test_markdown_string_escape_inline() {
        let input = "Text with #hashtag and -dash and *emphasis*";
        let expected = "Text with #hashtag and -dash and \\*emphasis\\*";
        assert_eq!(MarkdownString::escape_inline(input).0, expected);
    }

    #[test]
    #[allow(deprecated)]
    fn test_markdown_string_escape() {
        let input = r#"
        # Heading

        Another heading
        ===

        Another heading variant
        ---

        Paragraph with [link](https://example.com) and `code`, *emphasis*, and ~strikethrough~.

        ```
        code block
        ```

        List with varying leaders:
          - Item 1
          * Item 2
          + Item 3

        Some math:  $`\sqrt{3x-1}+(1+x)^2`$

        HTML entity: &nbsp;
        "#;

        let expected = r#"
        \# Heading

        Another heading
        \=\=\=

        Another heading variant
        \-\-\-

        Paragraph with \[link](https://example.com) and \`code\`, \*emphasis\*, and \~strikethrough\~.

        \`\`\`
        code block
        \`\`\`

        List with varying leaders:
          \- Item 1
          \* Item 2
          \+ Item 3

        Some math:  \$\`\\sqrt{3x\-1}\+(1\+x)\^2\`\$

        HTML entity: \&nbsp;
        "#;

        assert_eq!(MarkdownString::escape(input).0, expected);
    }

    #[test]
    fn test_markdown_string_inline_code() {
        assert_eq!(MarkdownString::inline_code(" ").0, "` `");
        assert_eq!(MarkdownString::inline_code("text").0, "`text`");
        assert_eq!(MarkdownString::inline_code("text ").0, "`text `");
        assert_eq!(MarkdownString::inline_code(" text ").0, "`  text  `");
        assert_eq!(MarkdownString::inline_code("`").0, "`` ` ``");
        assert_eq!(MarkdownString::inline_code("``").0, "``` `` ```");
        assert_eq!(MarkdownString::inline_code("`text`").0, "`` `text` ``");
        assert_eq!(
            MarkdownString::inline_code("some `text` no leading or trailing backticks").0,
            "``some `text` no leading or trailing backticks``"
        );
    }

    #[test]
    fn test_count_max_consecutive_chars() {
        assert_eq!(
            count_max_consecutive_chars("``a```b``", '`'),
            3,
            "the highest seen consecutive segment of backticks counts"
        );
        assert_eq!(
            count_max_consecutive_chars("```a``b`", '`'),
            3,
            "it can't be downgraded later"
        );
    }
}
