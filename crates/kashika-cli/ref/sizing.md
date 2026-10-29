# Sizing

Leave `size` out and a box fits its text: it gets the smallest size whose text area holds the label, sublabel, block and icon, rounded up to `grid=`. `size (140, auto)` fixes the width and fits the height. `kashika inspect file.kash <id>` shows the size it got. A box with no text still needs a `size`.

When you give a size, write a guess, run `kashika check`, and take the size from the `label-overflow` help. To work it out up front:

- `kashika measure "Payments API"` prints the exact width and height of a text (quote it; `\n` for more lines; several arguments give one result each). Add `--size N` and `--weight bold` to match the style; `**` markup is applied as in a label. `kashika ref metrics` has average character widths.
- A line of text is `1.25 × font-size` tall (17.5px at the default 14). A sublabel adds a `0.8 × font-size` line after `gap` (default 2). A block is as wide as its widest item.
- `padding` defaults to 8 on each side, so a `rect` or `rounded` box is the text plus 16 each way: `Payments API` is 91.64×17.5, so its box is 108×34.
- Other shapes hold their text in part of the box (`kashika ref shapes` lists each text area). A `diamond` uses 0.56 × 0.44 of the box and an `ellipse` 0.71 × 0.71: add the padding, then divide (`Payments API` needs a 193×77 diamond or a 153×48 ellipse). The rest lose pixels: a `hexagon` `min(w/4, h/4)` at each side, a `cylinder` up to 40px at the top (its cap), a `doc` up to 18px at the bottom (its wave), a `cloud` up to 20px each side and 36px of height, a `person` its head and shoulders, a `parallelogram` its slant (up to 24px) at each side, a `predefined-process` up to 12px and an `octagon` up to 12px at each side, a `note` up to 8px at the top and right, a `package` its tab (up to 12px) at the top, a `queue` up to 36px of width (its caps) and a `stadium` about 0.29 × its height at each end. A `triangle` uses the middle half of its lower half (0.5 × 0.5). One line of 14px text needs a cylinder about 67px tall, a cloud 61 and a person 74.
- `icon=` takes `icon-size + padding` (32px by default) from the icon's side.
- Small boxes, such as badges and table rows under ~30px tall, need a smaller inset: `padding=3` fits 12px text in a 22px row; `padding=(x,y)` sets left/right and top/bottom.
- With `grid=N`, round sizes up to the grid: `off-grid` checks every number you write for a position, distance or size.
