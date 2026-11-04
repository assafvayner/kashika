import skill from "../../../skills/kashika/SKILL.md?raw";
import type { Engine } from "../engine/types.js";

const APP_RULES = `# How you work in this app

You are the diagram agent in a web app. The user sees the rendered diagram and this chat, not the source.

- Never paste or quote Kashika source in your replies unless the user asks for it. Describe what changed in one or two sentences.
- The current source is given to you in each turn as a context block, so you do not need read_source unless you lost track.
- The only way to change the diagram is write_source with the complete new source. It returns the check report; read it.
- When the user says "this" or "it" and a selection is given as \`selected: <id>\`, that is the element they mean.
- Check reports look like \`diagram.kash:LINE:COL: error[code]: message\`, then the source line and \`= help:\` lines, ending with a summary \`N errors, M warnings · …\`. \`diagram.kash\` is the current source. Tool failures start with \`error:\`.
- There is no file system: \`import\`, \`image\`, \`shape … from "<file>"\`, \`icons="<dir>"\` and file icons do not work. Use built-in shapes and the bundled icons (find them with \`icons\`).
- Do not ask questions you can answer with a tool call. Do not describe your tool calls.

# Draft first, let the tools do the geometry

The checker, inspect, measure and route compute exact geometry in milliseconds. You do not: never work out pixel positions, anchor fractions, label widths or free lanes in your head, and never try to make a write check clean before sending it. A write with findings is normal; the user sees every write on the canvas and wants to see the diagram appear early and grow.

1. Plan in a few sentences at most: which boxes, in which rows and columns, which arrows. For a long brief, the boxes are the components and the arrows are the main flows; details that are not a box or an arrow go into one or two \`note\` boxes with short lines, or a short line label. Do not draw one box per sentence.
2. Your first tool call is write_source with the boxes and groups only, no lines: the first box with a literal \`at\`, every other box relative to a neighbor (\`at api.right+(80,0) pin=left\`, \`at api.bottom+(70,0) pin=top\`), no \`size\` on text boxes, \`ortho\` on the \`diagram\` line. Do not read ref topics before this write; the core guide below is enough.
3. Second write: add the arrows, each with one simple path (\`a.right -> b.left\`, \`a.right -> hv b.top\`) and a short label where the brief names one. Third write: notes, sublabels and styles.
4. Then fix in rounds: read the whole report, fix every finding whose help line names the fix, and write once per round, not once per finding. For a line with \`cross\`, \`through\` or \`near\`, call route and paste its suggestion instead of guessing a path. For alignment, call inspect and use the anchor it shows. Use ref only for a topic or code you need and do not know.
5. Fix every error first, then every warning. Stop when the check is clean, or explain in one sentence what remains and why you left it.
6. The budget is a fixed number of tool calls per turn; three staged writes plus three fix rounds fit. If the budget is near, write the best source you have and say what is left.
`;

/** Rewrites the skill's CLI commands as tool names. */
function toolify(md: string): string {
  return md
    .replace(/^(\| the core guide:[^\n|]*\| )`kashika ref` \|$/m, "$1below |")
    .replace("`diagram` comes first, then any `import`s.", "`diagram` comes first.")
    .replace("with an `icon=` or `image`;", "with a bundled `icon=`;")
    .replace(
      /`kashika ref <code>`, e\.g\. `kashika ref through`/g,
      '`ref("<code>")`, e.g. `ref("through")`',
    )
    .replace(/`kashika ref ([a-z-]+)`/g, '`ref("$1")`')
    .replace(/`kashika ref <code>`/g, '`ref("<code>")`')
    .replace(/`ref <code>`/g, '`ref("<code>")`')
    .replace(/`kashika ref`/g, "the core guide")
    .replace(/`kashika check file\.kash`/g, "`write_source` (it checks too) or `check()`")
    .replace(/`kashika route file\.kash a\.right b\.left`/g, '`route("a.right", "b.left")`')
    .replace(
      /When a fix needs coordinates, `kashika inspect file\.kash <ids> --ports`[^\n]*/g,
      "When a fix needs coordinates, `inspect` (with `ids` and `ports`) returns absolute boxes, anchors and line vertices, and `route` suggests ready-to-paste lines.",
    )
    .replace(
      /When the check is clean, `kashika render file\.kash -o file\.png` and look at the picture\./g,
      "When the check is clean, confirm with `inspect` that every arrow points the right way and the right boxes connect.",
    )
    .replace(/`kashika inspect file\.kash <ids>`/g, "`inspect([ids])`")
    .replace(/`kashika inspect file\.kash <id>`/g, "`inspect([id])`")
    .replace(/`inspect file\.kash <id>`/g, "`inspect([id])`")
    .replace(
      /\*\*Render and look:\*\* `kashika render[^\n]*/g,
      "**Review:** you cannot see the render. Use `inspect` to confirm arrows point the right way, the right boxes connect and spacing is even, then check again.",
    )
    .replace(
      /Then render and look, since a clean check is not a good diagram\./g,
      "A clean check is not a good diagram, so also confirm the structure with `inspect`.",
    )
    .replace(
      /Errors stop `render` \(unless `--force`\) and are listed first/g,
      "Errors are listed first",
    )
    .replace(/`kashika measure "Payments API"`/g, '`measure("Payments API")`')
    .replace(/`kashika icons --search database`/g, '`icons("database")`')
    .replace(/^import "<file\.kash>"[^\n]*\n/gm, "")
    .replace(/, `route` and `--corridors` ignore/g, " and `route` ignore")
    .replace(/^\|[^\n]*every command and flag[^\n]*\n/gm, "")
    .replace(/^\| you need \| run \|$/gm, "| you need | use |")
    .replace(/`kashika [^`]*`/g, (m) => m.replace("kashika ", ""));
}

const cache = new WeakMap<Engine, string>();

export function systemPrompt(engine: Engine): string {
  const cached = cache.get(engine);
  if (cached !== undefined) return cached;
  const skillBody = skill
    .replace(/^---\n[\s\S]*?\n---\n+/, "")
    .replace(
      "the `kashika` CLI checks the geometry and renders SVG or PNG. The full language reference is built into the tool: **run `kashika ref` first**, then the topics it lists as you need them.",
      "the app checks the geometry and renders it for the user. The core guide to the language is below; fetch the topics it lists with `ref` as you need them.",
    );
  const guide = toolify(engine.ref())
    .replace(
      /`ref <topic>` prints one of these; `ref\("<code>"\)` explains a diagnostic\./,
      '`ref(topic)` returns one of these; `ref("<code>")` explains a diagnostic.',
    )
    .replace(
      /^## The loop\n[\s\S]*?(?=^## )/m,
      'The loop is described above, under "Draft first".\n\n',
    );
  const prompt = [
    toolify(skillBody).replace(/^## The loop\n[\s\S]*?(?=^## )/m, ""),
    APP_RULES,
    "# Language reference (core guide)",
    guide,
  ].join("\n\n");
  cache.set(engine, prompt);
  return prompt;
}
