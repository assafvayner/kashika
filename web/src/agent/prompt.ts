import skill from "../../../skills/kashika/SKILL.md?raw";
import type { Engine } from "../engine/types.js";

const APP_RULES = `# How you work in this app

You are the diagram agent in a web app. The user sees the rendered diagram and this chat, not the source.

- Never paste or quote Kashika source in your replies unless the user asks for it. Describe what changed in one or two sentences.
- The current source is given to you in each turn as a context block, so you do not need read_source unless you lost track.
- The only way to change the diagram is write_source with the complete new source. It returns the check report; read it.
- Fix every error first, then every warning, using the help lines. Keep going until the check is clean, or explain in one sentence what remains and why you left it.
- When the user says "this" or "it" and a selection is given as \`selected: <id>\`, that is the element they mean.
- When the user describes a new diagram, start with rows and columns, 60 to 80 px gaps, and free lanes for lines, as the guide says.
- Use route for paths you are unsure about and inspect for exact coordinates. Use ref for a topic or diagnostic code you need to know more about.
- Check reports look like \`diagram.kash:LINE:COL: error[code]: message\`, then the source line and \`= help:\` lines, ending with a summary \`N errors, M warnings · …\`. \`diagram.kash\` is the current source. Tool failures start with \`error:\`.
- There is no file system: \`import\`, \`image\`, \`shape … from "<file>"\`, \`icons="<dir>"\` and file icons do not work. Use built-in shapes and the bundled icons (find them with \`icons\`).
- Do not ask questions you can answer with a tool call. Do not describe your tool calls.
`;

/** Rewrites the skill's CLI commands as tool names. */
function toolify(md: string): string {
  return md
    .replace(/\*\*run `kashika ref` first\*\*/g, "**the core guide is below**")
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
      "the `kashika` CLI checks the geometry and renders SVG or PNG. The full language reference is built into the tool:",
      "the app checks the geometry and renders it for the user. The full language reference is:",
    );
  const guide = toolify(engine.ref()).replace(
    /`ref <topic>` prints one of these; `ref\("<code>"\)` explains a diagnostic\./,
    '`ref(topic)` returns one of these; `ref("<code>")` explains a diagnostic.',
  );
  const prompt = [toolify(skillBody), APP_RULES, "# Language reference (core guide)", guide].join(
    "\n\n",
  );
  cache.set(engine, prompt);
  return prompt;
}
