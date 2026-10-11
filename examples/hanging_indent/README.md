Example showing a `TextEdit::multiline` with a custom layouter that gives markdown list items a hanging indent,
using `LayoutSection::hanging_indent`.

Wrapped rows of a list item, and lines continuing it after a hard break, line up with the text after the bullet.
The indent is purely visual: no characters are inserted into the text.

```sh
cargo run -p hanging_indent
```
