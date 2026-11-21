# wct-rs

Reader and writer for Warcraft III custom script files, `war3map.wct`:
the code World Editor's trigger editor keeps as text, the map's custom
script (its header) and each trigger converted to custom text. A
library, a small CLI and a text form: the code, one piece after another,
which the [wc3-rs](../wc3-rs) editor opens and its server keeps in the
project's model as code, like any script.

Reading and writing are exact: `to_bytes` gives back the bytes `parse`
read, and so does the text form, on every map of the game, a corpus of
custom maps and the maps in `maps/` (207 files; 17 triggers with code,
and the headers of the maps that have one).

## Format

Little-endian. Before 1.31:

| type | field |
|---|---|
| u32 | format version: 0 Reign of Chaos, 1 The Frozen Throne |
| string | (1) the comment on the map's custom script |
| code | (1) the map's custom script |
| u32 | the number of triggers |
| code × n | each trigger's code, in the trigger editor's order |

From 1.31 the file starts with `0x80000004`, then the version, the
comment and the custom script as above, and no count: the triggers'
code runs to the end of the file, one for each trigger that isn't a
comment (which `war3map.wtg` says).

A **code** is a u32 size, then that many bytes: the code and its NUL. A
size of 0 is no code at all, which is what every trigger that isn't
custom text stores. Code is kept as bytes, in the encoding the map was
saved in; World Editor breaks its lines with `\r\n`.

Every map at hand uses the layout before 1.31; the other is checked by
tests only.

## Library

```rust
let file = wct_rs::parse(&std::fs::read("war3map.wct")?)?;
println!("{}", String::from_utf8_lossy(file.header.text()));
for code in &file.triggers {
    // code.0 is None for a trigger without code
}
```

`Code::text` is the code without its NUL; `Code::new` makes one the way
World Editor stores it. Errors are `parser_rs::BinError`, with the byte
offset.

## Text

`text::to_text(file, Script::Jass)` and `text::from_text`: the code, one
piece after another, each after a marker line, a comment of the script's
language (`//`, or `--` in a Lua map) starting with `@wct`. So the text
reads, highlights and checks as the script it is.

```jass
// @wct version=1
// @wct comment "The map's own functions"
// @wct header
function Helper takes nothing returns nothing
endfunction
// @wct trigger
function Trig_Init_Actions takes nothing returns nothing
    call Helper()
endfunction
// @wct trigger none
```

- `version=0|1`, `magic` (the layout from 1.31) and `eol=lf` come first;
- `comment "..."` (version 1), with `\n`, `\r`, `\t`, `\"` and `\\`
  escapes;
- `header` (version 1): the map's custom script follows; `header none`
  stores none;
- `trigger`: a trigger's code follows, up to the next marker;
  `trigger none` stores no code.

Line breaks are `\n` in the text, `\r\n` again when saved, unless the
file says `eol=lf` (code that breaks lines with `\n` only). A line of
code that reads as a marker can't be shown. Code that isn't UTF-8 shows
with U+FFFD.

## CLI

```sh
wct-rs war3map.wct                       # read and write back: a check
wct-rs --text [--lua] war3map.wct        # the text form
wct-rs --from-text map.j war3map.wct     # text back to a file
```

## Tests

```sh
cargo test
WCT_CORPUS=<folder of map folders> cargo test --release corpus -- --ignored
```
