# Kashika reference

Kashika draws technical diagrams from `.kash` files. There is no auto-layout: you place every box and route every line, and `kashika check` reports what is wrong with the geometry. This page is the core guide; the topics at the end go deeper, and `kashika ref <code>` explains one diagnostic (`kashika ref through`).

## The loop

1. Sketch the layout as a grid: rows and columns with 60–80px gaps, and free lanes for the lines.
2. Write the file. Leave `size` off boxes so they fit their text (`kashika ref sizing`).
3. `kashika check file.kash`. Fix every error first, starting with the first, then every warning. The `help` lines name the fix: did-you-mean names, required sizes, clear lanes (`free`), `label@t` positions, exact anchors.
4. When a fix needs coordinates, `kashika inspect file.kash <ids> --ports` prints absolute boxes, anchors and line vertices, `--corridors x1,y1,x2,y2` lists free lanes, and `kashika route file.kash a.right b.left` suggests ready-to-paste lines (`kashika ref cli`).
5. When the check is clean, `kashika render file.kash -o file.png` and look at the picture. A clean check is not a good diagram.
