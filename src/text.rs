//! The text form of a custom script file: its code, one piece after
//! another, each after a marker line, so that it reads, highlights and
//! checks as the script it is (JASS, or Lua in a Lua map). The markers
//! are comments of the script's language (`//` or `--`) starting with
//! `@wct`:
//!
//! ```jass
//! // @wct version=1
//! // @wct comment "The map's own functions"
//! // @wct header
//! function Helper takes nothing returns nothing
//! endfunction
//! // @wct trigger
//! function Trig_Init_Actions takes nothing returns nothing
//! endfunction
//! // @wct trigger none
//! ```
//!
//! - `version=0|1`, `magic` for the layout from 1.31, `eol=lf` (the
//!   first lines, before any other marker);
//! - `comment "..."` (version 1): the comment on the map's custom
//!   script, with `\n`, `\r`, `\t`, `\"` and `\\` escapes;
//! - `header` (version 1): the map's custom script follows;
//! - `trigger`: a trigger's code follows; `trigger none` is one that
//!   stores no code at all (most of them: only custom text triggers have
//!   code), and has nothing after it.
//!
//! Code runs from the line after its marker to the line before the next
//! one. Line breaks are `\n` in the text; World Editor writes `\r\n`, and
//! saving turns them back into that, unless the file says `eol=lf`. The
//! text gives back the exact bytes of a file whose code is UTF-8 with one
//! kind of line break; other bytes show as U+FFFD.

use crate::{Code, CustomText};

/// The markers' word.
const WORD: &str = "@wct";

/// The script's language, for the markers' comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Script {
    #[default]
    Jass,
    Lua,
}

impl Script {
    fn comment(self) -> &'static str {
        match self {
            Script::Jass => "//",
            Script::Lua => "--",
        }
    }
}

fn escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\n' => out += "\\n",
            '\r' => out += "\\r",
            '\t' => out += "\\t",
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            c => out.push(c),
        }
    }
    out + "\""
}

fn unescape(s: &str) -> Result<String, String> {
    let inner = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or("the comment is a quoted string")?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next() {
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('"') => '"',
            Some('\\') => '\\',
            _ => return Err("the escapes are \\n \\r \\t \\\" \\\\".into()),
        });
    }
    Ok(out)
}

/// The text form of `file`, its markers comments of `script`. Fails when
/// code has a line that reads as a marker.
pub fn to_text(file: &CustomText, script: Script) -> Result<String, String> {
    let c = script.comment();
    let texts: Vec<String> = std::iter::once(&file.header)
        .chain(&file.triggers)
        .map(|code| String::from_utf8_lossy(code.text()).into_owned())
        .collect();
    // `\n` only if some code breaks lines with a bare `\n` and none with
    // `\r\n`.
    let crlf = texts.iter().any(|t| t.contains("\r\n"));
    let lf = texts.iter().any(|t| t.replace("\r\n", "").contains('\n'));

    let mut out = format!("{c} {WORD} version={}\n", file.version);
    if file.magic {
        out += &format!("{c} {WORD} magic\n");
    }
    if lf && !crlf {
        out += &format!("{c} {WORD} eol=lf\n");
    }
    let piece = |out: &mut String, marker: &str, code: &Code, text: &str| -> Result<(), String> {
        if code.0.is_none() {
            *out += &format!("{c} {WORD} {marker} none\n");
            return Ok(());
        }
        let text = text.replace("\r\n", "\n");
        if text.lines().any(is_marker) {
            return Err(format!("its code has a line that reads as a `{WORD}` marker"));
        }
        *out += &format!("{c} {WORD} {marker}\n{text}\n");
        Ok(())
    };
    if file.version == 1 {
        let comment = String::from_utf8_lossy(&file.comment);
        out += &format!("{c} {WORD} comment {}\n", escape(&comment));
        piece(&mut out, "header", &file.header, &texts[0])?;
    }
    for (code, text) in file.triggers.iter().zip(&texts[1..]) {
        piece(&mut out, "trigger", code, text)?;
    }
    Ok(out)
}

/// Is `line` a marker: a comment (`//` or `--`) starting with `@wct`?
pub fn is_marker(line: &str) -> bool {
    marker(line).is_some()
}

/// What follows `@wct` on a marker line.
fn marker(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = line.strip_prefix("//").or_else(|| line.strip_prefix("--"))?;
    let rest = rest.trim_start().strip_prefix(WORD)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

/// An error in the text: its line (1-based) and what's wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

/// Where the code being read goes.
enum Target {
    Header,
    Trigger,
}

/// Reads the text form back: the file, or every error, in order.
pub fn from_text(text: &str) -> Result<CustomText, Vec<Error>> {
    let text = text.replace("\r\n", "\n");
    let mut file = CustomText::default();
    let mut errors = Vec::new();
    let mut lf = false;
    let mut version = None;
    let mut comment: Option<String> = None;
    let mut header: Option<Code> = None;
    let mut current: Option<(Target, Vec<&str>)> = None;
    // Past the settings: a comment, a header or a trigger came.
    let mut past_settings = false;
    let finish = |current: &mut Option<(Target, Vec<&str>)>, file: &mut CustomText, header: &mut Option<Code>| {
        if let Some((target, lines)) = current.take() {
            let code = Code::new(lines.join("\n").as_bytes());
            match target {
                Target::Header => *header = Some(code),
                Target::Trigger => file.triggers.push(code),
            }
        }
    };
    let body = text.strip_suffix('\n').unwrap_or(&text);
    for (i, line) in body.split('\n').enumerate() {
        let n = i + 1;
        let Some(m) = marker(line) else {
            match &mut current {
                Some((_, lines)) => lines.push(line),
                None if line.trim().is_empty() => {}
                None => errors.push(Error {
                    line: n,
                    message: format!("code goes after a `{WORD} header` or `{WORD} trigger` line"),
                }),
            }
            continue;
        };
        finish(&mut current, &mut file, &mut header);
        let (word, rest) = m.split_once(char::is_whitespace).unwrap_or((m, ""));
        let rest = rest.trim();
        let mut err = |message: String| errors.push(Error { line: n, message });
        match word {
            w if w.starts_with("version=") && !past_settings => match &w["version=".len()..] {
                "0" => version = Some(0),
                "1" => version = Some(1),
                v => err(format!(
                    "unknown version {v}: 0 (Reign of Chaos) or 1 (The Frozen Throne)"
                )),
            },
            "magic" if rest.is_empty() && !past_settings => file.magic = true,
            "eol=lf" if !past_settings => lf = true,
            "eol=crlf" if !past_settings => lf = false,
            "comment" if comment.is_none() && header.is_none() && file.triggers.is_empty() => {
                past_settings = true;
                match unescape(rest) {
                    Ok(c) => comment = Some(c),
                    Err(e) => err(e),
                }
            }
            "header" if header.is_none() && file.triggers.is_empty() => {
                past_settings = true;
                match rest {
                    "" => current = Some((Target::Header, Vec::new())),
                    "none" => header = Some(Code(None)),
                    _ => err(format!("`{WORD} header`, or `{WORD} header none`")),
                }
            }
            "trigger" => {
                past_settings = true;
                match rest {
                    "" => current = Some((Target::Trigger, Vec::new())),
                    "none" => file.triggers.push(Code(None)),
                    _ => err(format!("`{WORD} trigger`, or `{WORD} trigger none`")),
                }
            }
            _ => err(format!(
                "`{WORD} {m}` isn't a marker here: version=, magic and eol= come first, then comment and header (version 1), then the triggers"
            )),
        }
    }
    finish(&mut current, &mut file, &mut header);
    file.version = version.unwrap_or(1);
    if file.version == 1 {
        file.comment = comment.unwrap_or_default().into_bytes();
        file.header = header.unwrap_or_else(|| Code::new(b""));
    } else if comment.is_some() || header.is_some() {
        errors.push(Error {
            line: 1,
            message: "version 0 has no comment and no header".into(),
        });
    }
    if !lf {
        for code in std::iter::once(&mut file.header).chain(&mut file.triggers) {
            if let Some(bytes) = &mut code.0 {
                *bytes = String::from_utf8_lossy(bytes).replace('\n', "\r\n").into_bytes();
            }
        }
    }
    if errors.is_empty() { Ok(file) } else { Err(errors) }
}

#[cfg(test)]
#[path = "text.test.rs"]
mod tests;
