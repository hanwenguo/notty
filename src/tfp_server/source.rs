use std::ops::Range;

use ropey::Rope;
use serde::{Deserialize, Serialize};
use typst_syntax::{FileId, LinkedNode, Source, SyntaxKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextChange {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

pub struct OpenSource {
    pub path: String,
    pub source: Source,
    rope: Rope,
    pub version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquationRecord {
    pub start: usize,
    pub end: usize,
    pub body_start: usize,
    pub body_end: usize,
    pub block: bool,
}

impl OpenSource {
    pub fn new(path: String, id: FileId, text: String, version: u64) -> Self {
        Self {
            path,
            source: Source::new(id, text.clone()),
            rope: Rope::from_str(&text),
            version,
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn char_len(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn char_to_byte(&self, offset: usize) -> Result<usize, String> {
        if offset > self.rope.len_chars() {
            return Err(format!(
                "invalid character offset {offset} for source length {}",
                self.rope.len_chars()
            ));
        }
        Ok(self.rope.char_to_byte(offset))
    }

    pub fn slice_chars(&self, start: usize, end: usize) -> Result<String, String> {
        if start > end || end > self.rope.len_chars() {
            return Err(format!(
                "invalid character range {start}..{end} for source length {}",
                self.rope.len_chars()
            ));
        }
        Ok(self.rope.slice(start..end).to_string())
    }

    pub fn equations(&self) -> Vec<EquationRecord> {
        let text = self.source.text();
        let mut records = vec![];
        walk(LinkedNode::new(self.source.root()), &mut |node| {
            if node.kind() != SyntaxKind::Equation {
                return;
            }
            let bytes = node.range();
            let slice = &text[bytes.clone()];
            let (Some(first), Some(last)) = (slice.find('$'), slice.rfind('$')) else {
                return;
            };
            if first == last {
                return;
            }
            let body_start_byte = bytes.start + first + 1;
            let body_end_byte = bytes.start + last;
            let body = &text[body_start_byte..body_end_byte];
            records.push(EquationRecord {
                start: self.rope.byte_to_char(bytes.start),
                end: self.rope.byte_to_char(bytes.end),
                body_start: self.rope.byte_to_char(body_start_byte),
                body_end: self.rope.byte_to_char(body_end_byte),
                block: body.starts_with(char::is_whitespace) || body.ends_with(char::is_whitespace),
            });
        });
        records
    }

    pub fn edit(&mut self, change: TextChange) -> Result<Range<usize>, String> {
        if change.start > change.end || change.end > self.rope.len_chars() {
            return Err(format!(
                "invalid character range {}..{} for source length {}",
                change.start,
                change.end,
                self.rope.len_chars()
            ));
        }
        let bytes = self.rope.char_to_byte(change.start)..self.rope.char_to_byte(change.end);
        let dirty = self.source.edit(bytes, &change.text);
        self.rope.remove(change.start..change.end);
        self.rope.insert(change.start, &change.text);
        let start = self.rope.byte_to_char(dirty.start);
        let end = self.rope.byte_to_char(dirty.end);
        Ok(self.rope.char_to_line(start)..self.rope.char_to_line(end))
    }

    pub fn full_sync(&mut self, text: String, version: u64) {
        self.source.replace(&text);
        self.rope = Rope::from_str(&text);
        self.version = version;
    }
}

fn walk(node: LinkedNode<'_>, visit: &mut impl FnMut(&LinkedNode<'_>)) {
    visit(&node);
    for child in node.children() {
        walk(child, visit);
    }
}

#[cfg(test)]
mod tests {
    use typst_syntax::Source;

    use super::*;

    #[test]
    fn edits_use_unicode_character_offsets() {
        let source = Source::detached("A世界e\u{301}🙂\r\n");
        let id = source.id();
        let mut open = OpenSource::new("main.typ".into(), id, source.text().into(), 4);
        open.edit(TextChange {
            start: 1,
            end: 3,
            text: "中".into(),
        })
        .unwrap();
        open.edit(TextChange {
            start: 4,
            end: 5,
            text: "🚀".into(),
        })
        .unwrap();
        assert_eq!(open.text(), "A中e\u{301}🚀\r\n");
        assert_eq!(open.source.text(), open.text());
    }

    #[test]
    fn rejects_invalid_ranges() {
        let source = Source::detached("abc");
        let mut open = OpenSource::new("main.typ".into(), source.id(), "abc".into(), 0);
        assert!(
            open.edit(TextChange {
                start: 2,
                end: 4,
                text: String::new()
            })
            .is_err()
        );
    }

    #[test]
    fn discovers_equations_with_unicode_character_offsets() {
        let text = "中 $x$ and $ y + 🙂 $";
        let source = Source::detached(text);
        let open = OpenSource::new("main.typ".into(), source.id(), text.into(), 2);
        assert_eq!(
            open.equations(),
            vec![
                EquationRecord {
                    start: 2,
                    end: 5,
                    body_start: 3,
                    body_end: 4,
                    block: false,
                },
                EquationRecord {
                    start: 10,
                    end: 19,
                    body_start: 11,
                    body_end: 18,
                    block: true,
                },
            ]
        );
    }
}
