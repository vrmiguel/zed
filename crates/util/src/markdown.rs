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
    pub fn escape(text: &str) -> Self {
        Self::escape_with_mode(text, true)
    }

    /// Escapes markdown special characters in inline mode, where block-level markers are not escaped.
    pub fn escape_inline(text: &str) -> Self {
        Self::escape_with_mode(text, false)
    }

    fn escape_with_mode(text: &str, escape_block_markers: bool) -> Self {
        let mut chunks = Vec::new();
        let mut start_of_unescaped = None;
        let mut at_line_start = true;

        for (ix, c) in text.char_indices() {
            if at_line_start && c.is_whitespace() {
                if start_of_unescaped.is_none() {
                    start_of_unescaped = Some(ix);
                }
                continue;
            }

            let needs_escape = match c {
                // Always escaped characters
                '\\' | '`' | '*' | '_' | '[' | '^' | '$' | '~' | '&' => true,
                // Block marker characters that need escaping only at start of line in block mode
                '#' | '+' | '=' | '-' => at_line_start && escape_block_markers,
                // Handle special HTML cases
                '<' => {
                    if let Some(start) = start_of_unescaped {
                        chunks.push(&text[start..ix]);
                    }
                    chunks.push("&lt;");
                    start_of_unescaped = None;
                    at_line_start = false;
                    continue;
                }
                '>' => {
                    if let Some(start) = start_of_unescaped {
                        chunks.push(&text[start..ix]);
                    }
                    chunks.push("&gt;");
                    start_of_unescaped = None;
                    at_line_start = false;
                    continue;
                }
                _ => false,
            };

            if needs_escape {
                if let Some(start) = start_of_unescaped {
                    chunks.push(&text[start..ix]);
                }
                chunks.push("\\");
                start_of_unescaped = Some(ix);
            } else if start_of_unescaped.is_none() {
                start_of_unescaped = Some(ix);
            }

            at_line_start = c == '\n';
        }

        if let Some(start) = start_of_unescaped {
            chunks.push(&text[start..]);
        }
        Self(chunks.concat())
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
    fn test_block_vs_inline_escape() {
        let input = "# Heading\n- List item\n+ Another item";
        let expected_block = r"\# Heading\n\- List item\n\+ Another item";
        let expected_inline = "# Heading\n- List item\n+ Another item";

        assert_eq!(MarkdownString::escape(input).0, expected_block);
        assert_eq!(MarkdownString::escape_inline(input).0, expected_inline);
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