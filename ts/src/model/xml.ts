import { type Result, err, ok } from "../diag/rust.js";

/// A small, tolerant XML reader and writer for SVG files: elements, attributes, text,
/// comments, CDATA, processing instructions and the doctype. Only the five predefined
/// entities and character references are decoded.

export interface XmlAttr {
  name: string;
  /// The decoded value.
  value: string;
}

export interface XmlElement {
  kind: "element";
  name: string;
  attrs: XmlAttr[];
  children: XmlNode[];
}

export type XmlNode =
  | XmlElement
  | { kind: "text"; text: string }
  | { kind: "cdata"; text: string }
  | { kind: "comment"; text: string }
  | { kind: "pi"; text: string }
  | { kind: "doctype"; text: string };

export interface XmlDocument {
  /// The top-level nodes, exactly one of which is `root`.
  nodes: XmlNode[];
  root: XmlElement;
}

const PREDEFINED: Readonly<Record<string, string>> = {
  lt: "<",
  gt: ">",
  amp: "&",
  quot: '"',
  apos: "'",
};

function decode(s: string): string {
  return s.replace(/&(#x[0-9a-fA-F]+|#[0-9]+|[A-Za-z][A-Za-z0-9._-]*);/g, (m, e: string) => {
    if (e.startsWith("#x")) {
      const cp = parseInt(e.slice(2), 16);
      return cp <= 0x10ffff ? String.fromCodePoint(cp) : m;
    }
    if (e.startsWith("#")) {
      const cp = parseInt(e.slice(1), 10);
      return cp <= 0x10ffff ? String.fromCodePoint(cp) : m;
    }
    return PREDEFINED[e] ?? m;
  });
}

const NAME = /^[A-Za-z_:][A-Za-z0-9._:\-·À-￿]*/;

export function parseXml(src: string): Result<XmlDocument, string> {
  const s = src.startsWith("﻿") ? src.slice(1) : src;
  const top: XmlNode[] = [];
  const stack: XmlElement[] = [];
  let root: XmlElement | null = null;
  let i = 0;
  const push = (n: XmlNode) => {
    const parent = stack[stack.length - 1];
    if (parent !== undefined) parent.children.push(n);
    else top.push(n);
  };
  const until = (from: number, pat: string, what: string): Result<number, string> => {
    const j = s.indexOf(pat, from);
    return j < 0 ? err(`unclosed ${what}`) : ok(j);
  };
  while (i < s.length) {
    const lt = s.indexOf("<", i);
    const textEnd = lt < 0 ? s.length : lt;
    if (textEnd > i) {
      const text = s.slice(i, textEnd);
      if (stack.length > 0) push({ kind: "text", text: decode(text) });
      else if (text.trim() !== "") return err("text outside the root element");
      i = textEnd;
      continue;
    }
    if (s.startsWith("<!--", i)) {
      const j = until(i + 4, "-->", "comment");
      if (!j.ok) return j;
      push({ kind: "comment", text: s.slice(i + 4, j.value) });
      i = j.value + 3;
    } else if (s.startsWith("<![CDATA[", i)) {
      if (stack.length === 0) return err("CDATA outside the root element");
      const j = until(i + 9, "]]>", "CDATA section");
      if (!j.ok) return j;
      push({ kind: "cdata", text: s.slice(i + 9, j.value) });
      i = j.value + 3;
    } else if (s.startsWith("<?", i)) {
      const j = until(i + 2, "?>", "processing instruction");
      if (!j.ok) return j;
      push({ kind: "pi", text: s.slice(i + 2, j.value) });
      i = j.value + 2;
    } else if (s.startsWith("<!", i)) {
      if (stack.length > 0 || root !== null) return err("unexpected declaration");
      let j = i + 2;
      let depth = 0;
      let quote: string | null = null;
      for (; j < s.length; j++) {
        const c = s[j]!;
        if (quote !== null) {
          if (c === quote) quote = null;
        } else if (c === '"' || c === "'") quote = c;
        else if (c === "[") depth++;
        else if (c === "]") depth--;
        else if (c === ">" && depth <= 0) break;
      }
      if (j >= s.length) return err("unclosed declaration");
      push({ kind: "doctype", text: s.slice(i + 2, j) });
      i = j + 1;
    } else if (s.startsWith("</", i)) {
      const m = NAME.exec(s.slice(i + 2));
      if (m === null) return err("invalid end tag");
      let j = i + 2 + m[0].length;
      while (j < s.length && /\s/.test(s[j]!)) j++;
      if (s[j] !== ">") return err("invalid end tag");
      const open = stack.pop();
      if (open === undefined || open.name !== m[0]) {
        return err(`unexpected close tag \`${m[0]}\``);
      }
      i = j + 1;
    } else {
      const m = NAME.exec(s.slice(i + 1));
      if (m === null) return err("invalid element name");
      if (stack.length === 0 && root !== null) return err("more than one root element");
      const el: XmlElement = { kind: "element", name: m[0], attrs: [], children: [] };
      let j = i + 1 + m[0].length;
      let selfClosing = false;
      for (;;) {
        const ws = j;
        while (j < s.length && /\s/.test(s[j]!)) j++;
        if (j >= s.length) return err(`the element \`${el.name}\` was opened but never closed`);
        if (s[j] === ">") {
          j++;
          break;
        }
        if (s.startsWith("/>", j)) {
          selfClosing = true;
          j += 2;
          break;
        }
        if (j === ws) return err(`expected a space before an attribute of \`${el.name}\``);
        const an = NAME.exec(s.slice(j));
        if (an === null) return err(`invalid attribute in \`${el.name}\``);
        j += an[0].length;
        while (j < s.length && /\s/.test(s[j]!)) j++;
        if (s[j] !== "=") return err(`expected \`=\` after the attribute \`${an[0]}\``);
        j++;
        while (j < s.length && /\s/.test(s[j]!)) j++;
        const q = s[j];
        if (q !== '"' && q !== "'") return err(`expected a quote for the attribute \`${an[0]}\``);
        const end = s.indexOf(q, j + 1);
        if (end < 0) return err(`unclosed value of the attribute \`${an[0]}\``);
        const raw = s.slice(j + 1, end);
        if (raw.includes("<")) return err(`\`<\` in the value of the attribute \`${an[0]}\``);
        if (el.attrs.some((a) => a.name === an[0])) {
          return err(`duplicate attribute \`${an[0]}\``);
        }
        el.attrs.push({ name: an[0], value: decode(raw) });
        j = end + 1;
      }
      push(el);
      if (stack.length === 0) root = el;
      if (!selfClosing) stack.push(el);
      i = j;
    }
  }
  if (stack.length > 0) {
    return err(`the element \`${stack[stack.length - 1]!.name}\` was opened but never closed`);
  }
  if (root === null) return err("no root element");
  return ok({ nodes: top, root });
}

function escapeText(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function escapeAttr(s: string): string {
  return escapeText(s).replace(/"/g, "&quot;");
}

export function serializeXml(doc: XmlDocument): string {
  let out = "";
  const write = (n: XmlNode) => {
    switch (n.kind) {
      case "element": {
        out += `<${n.name}`;
        for (const a of n.attrs) out += ` ${a.name}="${escapeAttr(a.value)}"`;
        if (n.children.length === 0) {
          out += "/>";
        } else {
          out += ">";
          for (const c of n.children) write(c);
          out += `</${n.name}>`;
        }
        break;
      }
      case "text":
        out += escapeText(n.text);
        break;
      case "cdata":
        out += `<![CDATA[${n.text}]]>`;
        break;
      case "comment":
      case "doctype":
        break;
      case "pi":
        if (n.text.startsWith("xml ") || n.text === "xml") out += `<?${n.text}?>`;
        break;
    }
  };
  for (const n of doc.nodes) write(n);
  return out;
}

export function attr(el: XmlElement, name: string): string | null {
  return el.attrs.find((a) => a.name === name)?.value ?? null;
}

/// Every element in document order, `el` first.
export function descendants(el: XmlElement): XmlElement[] {
  const out: XmlElement[] = [];
  const walk = (e: XmlElement) => {
    out.push(e);
    for (const c of e.children) if (c.kind === "element") walk(c);
  };
  walk(el);
  return out;
}
