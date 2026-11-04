# Text metrics

Kashika measures text with its bundled fonts (Inter Regular and Bold, JetBrains Mono for `` `code` `` and box drawing, and Twemoji for emoji, most 1 em wide), so sizes are the same on every machine. For an exact size, run `kashika measure "text" [--size N] [--weight bold]`. Line height is 1.25 × font-size, a sublabel is 0.8 × font-size, and `padding` defaults to 8. `label-overflow` reports the size a box needs, and a box without `size` gets exactly that size. Average character widths, in px:
