import skill from "../../../skills/kashika/SKILL.md?raw";
import type { Engine } from "../engine/types.js";

const APP_RULES = `# How you work in this app

You are the diagram agent in a web app. The user sees the rendered diagram and this chat, not the source.

- Never paste or quote Kashika source in your replies unless the user asks for it. Describe what changed in one or two sentences.
- The current source is given to you in each turn as a context block, so you do not need read_source unless you lost track.
- The diagram changes only through write_source (the complete new source) and edit_source (exact find/replace edits and appended lines). Both return the check report; read it. Prefer edit_source: it costs a few lines instead of the whole file.
- When the user says "this" or "it" and a selection is given as \`selected: <id>\`, that is the element they mean.
- Check reports look like \`diagram.kash:LINE:COL: error[code]: message\`, then the source line and \`= help:\` lines, ending with a summary \`N errors, M warnings · …\`. \`diagram.kash\` is the current source. Tool failures start with \`error:\`.
- There is no file system: \`import\`, \`image\`, \`shape … from "<file>"\`, \`icons="<dir>"\` and file icons do not work. Use built-in shapes and the bundled icons (find them with \`icons\`).
- Do not ask questions you can answer with a tool call. Do not describe your tool calls.

# Draft first, let the tools do the geometry

The checker, inspect, measure and route compute exact geometry in milliseconds. You do not: never work out pixel positions, anchor fractions, label widths or free lanes in your head, and never try to make a write check clean before sending it. A write with findings is normal; the user sees every write on the canvas and wants to see the diagram appear early and grow.

1. Plan in a few sentences at most: which boxes, in which rows and columns, which arrows. For a long brief, the boxes are the components and the arrows are the main flows; details that are not a box or an arrow go into one or two \`note\` boxes with short lines, or a short line label. Do not draw one box per sentence.
2. Your first tool call is write_source with the boxes and groups only, no lines: the first box with a literal \`at\`, every other box relative to a neighbor (\`at api.right+(80,0) pin=left\`, \`at api.bottom+(70,0) pin=top\`), no \`size\` on text boxes. Do not read ref topics before this write; the core guide below is enough.
3. Second call: edit_source with \`append\` adding the arrows, each named (\`line api_db api.right -> db.left\`) with one simple path and a short label where the brief names one; do not try to make them clean. Choose the line style that suits the diagram: straight segments, elbows, \`curve\`, rounded corners with \`radius=\`, or \`ortho\` when the diagram wants strictly orthogonal routing; the paths section below lists them all, and nothing here prefers one. Third call: route with no arguments, with \`style\` arc or bezier when the diagram uses curves. It reroutes every line the check flags and returns one ready-to-paste statement per line; replace them all in one edit_source call, one edit per line. Then append notes, sublabels and styles.
4. Then fix in rounds: read the whole report and fix every finding whose help line names the fix in one edit_source call with one edit per finding, not one call per finding. Use write_source again only to restructure the layout. Lines with \`not-ortho\`, \`cross\`, \`through\` or \`near\` go to one route call with no arguments, never to guessed paths. For alignment, call inspect and use the anchor it shows. Use ref only for a topic or code you need and do not know.
5. Fix every error first, then every warning. The turn ends at the first clean check after the notes are in: reply in one or two sentences. Do not inspect, re-check or polish a clean diagram, and do not add things the brief did not ask for.
6. The budget is a fixed number of tool calls per turn; a draft, two appends, one route and three fix rounds fit. If the budget is near, write the best source you have and say what is left.
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
      "When the check is clean, you are done.",
    )
    .replace(/`kashika inspect file\.kash <ids>`/g, "`inspect([ids])`")
    .replace(/`kashika inspect file\.kash <id>`/g, "`inspect([id])`")
    .replace(/`inspect file\.kash <id>`/g, "`inspect([id])`")
    .replace(
      /\*\*Render and look:\*\* `kashika render[^\n]*/g,
      "**Done:** when the check is clean, reply. You cannot see the render, and `inspect` is for fixing a finding, not for admiring a clean diagram.",
    )
    .replace(
      /Then render and look, since a clean check is not a good diagram\./g,
      "A clean check means you are done.",
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
