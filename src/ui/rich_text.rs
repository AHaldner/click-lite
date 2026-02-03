use gpui::{
    AnyElement, App, ElementId, FontStyle, FontWeight, HighlightStyle, Hsla, InteractiveText,
    IntoElement, SharedString, StrikethroughStyle, StyledText, UnderlineStyle, Window,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;

static LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\s*([^\]]*?)\s*\]\(([^)]+)\)").expect("Invalid regex"));

#[derive(Clone, Default)]
pub struct RichText {
    pub text: SharedString,
    pub highlights: Vec<(Range<usize>, HighlightStyle)>,
    pub link_ranges: Vec<Range<usize>>,
    pub link_urls: Vec<String>,
}

impl RichText {
    pub fn from_markdown(content: &str, code_color: Hsla, link_color: Hsla) -> Self {
        let content = normalize_markdown(content);

        let mut text = String::new();
        let mut highlights = Vec::new();
        let mut link_ranges = Vec::new();
        let mut link_urls = Vec::new();

        parse_markdown_into(
            &content,
            &mut text,
            &mut highlights,
            &mut link_ranges,
            &mut link_urls,
            code_color,
            link_color,
        );

        let trimmed_len = text.trim_end().len();
        text.truncate(trimmed_len);

        for (range, _) in &mut highlights {
            if range.end > trimmed_len {
                range.end = trimmed_len;
            }
            if range.start > trimmed_len {
                range.start = trimmed_len;
            }
        }
        highlights.retain(|(range, _)| range.start < range.end);

        for range in &mut link_ranges {
            if range.end > trimmed_len {
                range.end = trimmed_len;
            }
            if range.start > trimmed_len {
                range.start = trimmed_len;
            }
        }

        RichText {
            text: SharedString::from(text),
            highlights,
            link_ranges,
            link_urls,
        }
    }

    pub fn element(
        &self,
        id: impl Into<ElementId>,
        base_color: Hsla,
        window: &mut Window,
        _cx: &App,
    ) -> AnyElement {
        let mut text_style = window.text_style();
        text_style.color = base_color;

        let styled_text = StyledText::new(self.text.clone())
            .with_default_highlights(&text_style, self.highlights.iter().cloned());

        InteractiveText::new(id, styled_text)
            .on_click(self.link_ranges.clone(), {
                let link_urls = self.link_urls.clone();
                move |ix, _, cx| {
                    if let Some(url) = link_urls.get(ix) {
                        if url.starts_with("http") {
                            cx.open_url(url);
                        }
                    }
                }
            })
            .into_any_element()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

fn parse_markdown_into(
    content: &str,
    text: &mut String,
    highlights: &mut Vec<(Range<usize>, HighlightStyle)>,
    link_ranges: &mut Vec<Range<usize>>,
    link_urls: &mut Vec<String>,
    code_color: Hsla,
    link_color: Hsla,
) {
    let mut bold_depth = 0;
    let mut italic_depth = 0;
    let mut strikethrough_depth = 0;
    let mut link_url: Option<String> = None;
    let mut in_code_block = false;
    let mut list_stack: Vec<(Option<u64>, bool)> = Vec::new();

    let options = Options::all();

    for (event, _source_range) in Parser::new_ext(content, options).into_offset_iter() {
        let prev_len = text.len();

        match event {
            Event::Text(t) => {
                text.push_str(t.as_ref());

                let mut style = HighlightStyle::default();

                if in_code_block {
                    style.background_color = Some(code_color);
                }
                if bold_depth > 0 {
                    style.font_weight = Some(FontWeight::BOLD);
                }
                if italic_depth > 0 {
                    style.font_style = Some(FontStyle::Italic);
                }
                if strikethrough_depth > 0 {
                    style.strikethrough = Some(StrikethroughStyle {
                        thickness: 1.0.into(),
                        ..Default::default()
                    });
                }
                if let Some(url) = link_url.clone() {
                    link_ranges.push(prev_len..text.len());
                    link_urls.push(url);
                    style.color = Some(link_color);
                    style.underline = Some(UnderlineStyle {
                        thickness: 1.0.into(),
                        ..Default::default()
                    });
                }

                if style != HighlightStyle::default() {
                    let mut merged = false;

                    if let Some((last_range, last_style)) = highlights.last_mut() {
                        if last_range.end == prev_len && last_style == &style {
                            last_range.end = text.len();
                            merged = true;
                        }
                    }

                    if !merged {
                        highlights.push((prev_len..text.len(), style));
                    }
                }
            }

            Event::Code(t) => {
                text.push_str(t.as_ref());

                let mut style = HighlightStyle {
                    background_color: Some(code_color),
                    ..Default::default()
                };

                if let Some(url) = link_url.clone() {
                    link_ranges.push(prev_len..text.len());
                    link_urls.push(url);
                    style.underline = Some(UnderlineStyle {
                        thickness: 1.0.into(),
                        ..Default::default()
                    });
                }

                highlights.push((prev_len..text.len(), style));
            }

            Event::Start(tag) => match tag {
                Tag::Paragraph => new_paragraph(text, &list_stack),
                Tag::Heading { .. } => {
                    new_paragraph(text, &list_stack);
                    bold_depth += 1;
                }
                Tag::CodeBlock(kind) => {
                    new_paragraph(text, &list_stack);
                    in_code_block = true;
                    let _ = kind;
                }
                Tag::Emphasis => italic_depth += 1,
                Tag::Strong => bold_depth += 1,
                Tag::Strikethrough => strikethrough_depth += 1,
                Tag::Link { dest_url, .. } => {
                    link_url = Some(dest_url.to_string());
                }
                Tag::List(number) => {
                    list_stack.push((number, false));
                }
                Tag::Item => {
                    let len = list_stack.len();
                    if let Some((list_number, has_content)) = list_stack.last_mut() {
                        *has_content = false;
                        if !text.is_empty() && !text.ends_with('\n') {
                            text.push('\n');
                        }

                        for _ in 0..len.saturating_sub(1) {
                            text.push_str("  ");
                        }

                        if let Some(number) = list_number {
                            text.push_str(&format!("{}. ", number));
                            *number += 1;
                        } else {
                            text.push_str("• ");
                        }
                    }
                }
                Tag::BlockQuote(_) => {
                    new_paragraph(text, &list_stack);
                    text.push_str("> ");
                }
                _ => {}
            },

            Event::End(tag) => match tag {
                TagEnd::Heading(_) => bold_depth -= 1,
                TagEnd::CodeBlock => {
                    in_code_block = false;
                }
                TagEnd::Emphasis => italic_depth -= 1,
                TagEnd::Strong => bold_depth -= 1,
                TagEnd::Strikethrough => strikethrough_depth -= 1,
                TagEnd::Link => link_url = None,
                TagEnd::List(_) => {
                    list_stack.pop();
                }
                TagEnd::Item => {
                    if let Some((_, has_content)) = list_stack.last_mut() {
                        *has_content = true;
                    }
                }
                _ => {}
            },

            Event::HardBreak => text.push('\n'),
            Event::SoftBreak => text.push('\n'),
            Event::Rule => {
                new_paragraph(text, &list_stack);
                text.push_str("───────────────");
            }
            _ => {}
        }
    }
}

fn new_paragraph(text: &mut String, list_stack: &[(Option<u64>, bool)]) {
    if let Some((_, has_content)) = list_stack.last() {
        if !*has_content {
            return;
        }
    }

    if !text.is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
    }
}

fn normalize_markdown(content: &str) -> String {
    let content = fix_clickup_links(content);
    content
}

fn fix_clickup_links(content: &str) -> String {
    let result = LINK_REGEX.replace_all(content, |caps: &regex::Captures| {
        let display_text = &caps[1];
        let url = &caps[2];

        let clean_display: String = display_text
            .replace("\\_", "_")
            .replace("\\", "")
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let final_display = if clean_display.contains("http") {
            clean_display
                .split_whitespace()
                .next()
                .unwrap_or(&clean_display)
                .to_string()
        } else {
            clean_display
        };

        let clean_url = url.replace("\\_", "_").replace("\\", "");

        format!("[{}]({})", final_display, clean_url)
    });

    result.into_owned()
}
