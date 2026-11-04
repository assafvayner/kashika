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
- Do not ask questions you can answer with a tool call. Do not describe your tool calls.
`;

/** Rewrites the skill's CLI commands as tool names. */
function toolify(md: string): string {
  return md
    .replace(/\*\*run `kashika ref` first\*\*/g, "**the core guide is below**")
    .replace(/`kashika ref ([a-z-]+)`/g, '`ref("$1")`')
    .replace(
      /`kashika ref <code>`, e\.g\. `kashika ref through`/g,
      '`ref("<code>")`, e.g. `ref("through")`',
    )
    .replace(/`kashika ref`/g, "the core guide")
    .replace(/`kashika check file\.kash`/g, "`write_source` (it checks too) or `check()`")
    .replace(/`kashika route file\.kash a\.right b\.left`/g, '`route("a.right", "b.left")`')
    .replace(/`kashika inspect file\.kash <ids>`/g, "`inspect([ids])`")
    .replace(
      /`kashika render file\.kash -o file\.png`, then open or read the PNG/g,
      "the user sees the render",
    )
    .replace(/`kashika measure "Payments API"`/g, '`measure("Payments API")`')
    .replace(/`kashika icons --search database`/g, '`icons("database")`')
    .replace(/\| every command and flag[^\n]*\n/g, "")
    .replace(/`kashika [^`]*`/g, (m) => m.replace("kashika ", ""));
}

export function systemPrompt(engine: Engine): string {
  const guide = toolify(engine.ref()).replace(
    /`ref <topic>` prints one of these; `ref <code>` explains a diagnostic\./,
    "`ref(topic)` returns one of these; `ref(code)` explains a diagnostic.",
  );
  return [toolify(skill), APP_RULES, "# Language reference (core guide)", guide].join("\n\n");
}
