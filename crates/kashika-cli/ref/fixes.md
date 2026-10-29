## Fixing diagnostics

Errors stop `render` (unless `--force`) and are listed first; fix them before warnings. `kashika ref lints` lists every code. The fixes needed most:

- `parse`: fix these first. A statement that fails to parse is dropped, but references to its id are not reported again as `unknown-id`.
- `label-overflow`: use the size in the help, shorten the text, break it with `\n`, lower `padding=` on a small box, or leave `size` out.
- `cross` / `through` / `near`: move the segment to a lane from the `free` help, or ask `kashika route`. `allow=cross:<other>` only for a non-planar graph. `through` or `near` on a line's own endpoint box means the elbow is the wrong way round: enter a `left`/`right` side with `vh` or an `x …` hop, and a `top`/`bottom` side with `hv` or a `y …` hop.
- `shared-segment` / `port-crowding`: spread the ports with `side@t` (`core.right@0.3`, `core.right@0.7`), or fan out from a `junction`.
- `not-ortho` / `diagonal-tail`: finish with `hv`/`vh` or `x`/`y`.
- `label-collision`: move a line label to a `label@t` from the help, widen the gap to the run the help names, or use `label-side`. Route lines below a group's title band, or move the title with `text-align=right` on the group. A free `text` over a box goes into the box's block.
- `near-axis` / `jog`: a slant or a small step between runs a few px apart; use the level anchor from the help (`cdn.left@0.42`), or `hv`/`vh`.
- `out-of-canvas`: leave `size` off the `diagram` line so the canvas fits, and keep content at x, y ≥ 0.
- `missing-glyph`: replace CJK and similar characters with an `icon=` or `image`; emoji and box drawing are covered.
- `text-conflict`: move `label`/`sublabel` into the block as its first items, or give every block item an `at`.
