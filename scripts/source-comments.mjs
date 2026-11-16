import { readFileSync, readdirSync, statSync, mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, relative, extname, basename } from "node:path";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { parse as parseJavaScript, parseExpression } from "@babel/parser";
import { parse as parseVue } from "@vue/compiler-sfc";
import { parse as parseHtml } from "@vue/compiler-dom";
import postcss from "postcss";

export const repository = resolve(import.meta.dirname, "..");
export const policy = JSON.parse(readFileSync(resolve(import.meta.dirname, "comment-policy.json"), "utf8"));

export function sources(root = repository) {
  const found = new Set();
  function visit(file) {
    const name = relative(root, file).replaceAll("\\", "/");
    if (policy.thirdPartyFiles.includes(name) || policy.thirdPartyDirectories.some(dir => name === dir || name.startsWith(dir + "/"))) return;
    const stat = statSync(file, { throwIfNoEntry: false });
    if (!stat) return;
    if (stat.isDirectory()) {
      if (policy.ignoredDirectories.includes(basename(file))) return;
      for (const child of readdirSync(file)) visit(resolve(file, child));
    } else if (stat.isFile() && policy.extensions.includes(extname(file))) found.add(name);
  }
  for (const entry of policy.paths) visit(resolve(root, entry));
  return [...found].sort();
}

function cComments(text, rust, kotlin = false) {
  const spans = [];
  function string(i, delimiter, interpolate) {
    i += delimiter.length;
    while (i < text.length) {
      if (interpolate && text.startsWith("${", i)) i = scan(i + 2, true);
      else if (text[i] === "\\" && !(kotlin && delimiter.length === 3)) i += 2;
      else if (text.startsWith(delimiter, i)) return i + delimiter.length;
      else i++;
    }
    throw new Error("Unclosed string");
  }
  function scan(i = 0, interpolation = false) {
    let braces = 0;
    while (i < text.length) {
      const start = i;
      if (text.startsWith("//", i)) {
        const end = text.indexOf("\n", i);
        i = end < 0 ? text.length : end;
        spans.push([start, text[i - 1] === "\r" ? i - 1 : i]);
      } else if (text.startsWith("/*", i)) {
        let depth = 1;
        i += 2;
        while (i < text.length && depth) {
          if (text.startsWith("*/", i)) { depth--; i += 2; }
          else if (text.startsWith("/*", i)) { depth++; i += 2; }
          else i++;
        }
        if (depth) throw new Error("Unclosed block comment");
        spans.push([start, i]);
      } else if (rust && /^(?:br|cr|r)(#*)"/.test(text.slice(i))) {
        const match = text.slice(i).match(/^(?:br|cr|r)(#*)"/);
        const end = text.indexOf('"' + match[1], i + match[0].length);
        if (end < 0) throw new Error("Unclosed raw string");
        i = end + 1 + match[1].length;
      } else if (!rust && text.startsWith('"""', i)) {
        i = string(i, '"""', kotlin);
      } else if (text[i] === '"' || (text[i] === "'" && (!rust || /^'(?:[^'\\\r\n]|\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.))'/u.test(text.slice(i))))) {
        i = string(i, text[i], kotlin && text[i] === '"');
      } else if (interpolation && text[i] === "}" && braces === 0) {
        return i + 1;
      } else if (interpolation && text[i] === "{") {
        braces++; i++;
      } else if (interpolation && text[i] === "}") {
        braces--; i++;
      } else i++;
    }
    if (interpolation) throw new Error("Unclosed string interpolation");
    return i;
  }
  scan();
  return spans;
}

function jsComments(text, file) {
  const plugins = [];
  if (/\.(?:ts|tsx)$/.test(file)) plugins.push(["typescript", { dts: file.endsWith(".d.ts") }]);
  if (/\.[jt]sx$/.test(file)) plugins.push("jsx");
  return parseJavaScript(text, { sourceType: "unambiguous", plugins }).comments.map(c => [c.start, c.end]);
}

function cssComments(text) {
  const spans = [];
  postcss.parse(text).walkComments(node => spans.push([node.source.start.offset, node.source.end.offset]));
  return spans;
}

function htmlComments(text, embedded = true) {
  const spans = [];
  function expression(node, statement = false) {
    if (!node || !/[\/]\*|\/\//.test(node.content)) return;
    const parsed = statement ? parseJavaScript(node.content, { plugins: ["typescript"] }) : parseExpression(node.content, { plugins: ["typescript"] });
    const offset = node.loc.start.offset;
    spans.push(...(parsed.comments ?? []).map(comment => [offset + comment.start, offset + comment.end]));
  }
  function visit(node) {
    if (node.type === 3) spans.push([node.loc.start.offset, node.loc.end.offset]);
    if (node.type === 5) expression(node.content);
    for (const prop of node.props ?? []) if (prop.type === 7) {
      if (prop.arg && !prop.arg.isStatic) expression(prop.arg);
      if (!prop.exp) continue;
      if (prop.name === "for") {
        const match = prop.exp.content.match(/^(.*?)\s+(?:in|of)\s+([\s\S]+)$/);
        if (match) {
          const rhs = prop.exp.content.lastIndexOf(match[2]);
          expression({ content: match[2], loc: { start: { offset: prop.exp.loc.start.offset + rhs } } });
          const alias = match[1].replace(/^\s*\(/, "").replace(/\)\s*$/, "");
          const aliasOffset = prop.exp.content.indexOf(alias);
          const parsed = parseExpression(`(${alias}) => {}`, { plugins: ["typescript"] });
          spans.push(...(parsed.comments ?? []).map(c => [prop.exp.loc.start.offset + aliasOffset + c.start - 1, prop.exp.loc.start.offset + aliasOffset + c.end - 1]));
        }
      } else if (prop.name === "slot") {
        const parsed = parseExpression(`(${prop.exp.content}) => {}`, { plugins: ["typescript"] });
        spans.push(...(parsed.comments ?? []).map(c => [prop.exp.loc.start.offset + c.start - 1, prop.exp.loc.start.offset + c.end - 1]));
      } else expression(prop.exp, prop.name === "on");
    }
    if (embedded && node.type === 1 && ["script", "style"].includes(node.tag)) {
      const raw = node.children.map(child => child.content ?? "").join("");
      const offset = node.loc.start.offset + text.slice(node.loc.start.offset, node.loc.end.offset).indexOf(">") + 1;
      spans.push(...(node.tag === "script" ? jsComments(raw, ".js") : cssComments(raw)).map(([a, b]) => [a + offset, b + offset]));
    } else for (const child of node.children ?? []) visit(child);
  }
  visit(parseHtml(text, { comments: true, onError: error => { throw error; } }));
  return spans;
}

function vueComments(text, file) {
  const { descriptor, errors } = parseVue(text, { filename: file });
  if (errors.length) throw new Error(errors.join("; "));
  const spans = [];
  for (const block of [descriptor.script, descriptor.scriptSetup]) if (block) {
    spans.push(...jsComments(block.content, `script.${block.lang || "js"}`).map(([a, b]) => [a + block.loc.start.offset, b + block.loc.start.offset]));
  }
  if (descriptor.template) {
    const block = descriptor.template;
    spans.push(...htmlComments(block.content, false).map(([a, b]) => [a + block.loc.start.offset, b + block.loc.start.offset]));
  }
  for (const block of descriptor.styles) spans.push(...cssComments(block.content).map(([a, b]) => [a + block.loc.start.offset, b + block.loc.start.offset]));
  const ast = parseHtml(text, { comments: true, onError: () => {} });
  for (const child of ast.children) if (child.type === 3) spans.push([child.loc.start.offset, child.loc.end.offset]);
  return spans;
}

function shellComments(text) {
  const spans = [];
  const commands = [];
  let offset = 0, quote = null, heredoc = null, parameters = 0;
  function command(end) {
    commands.push({ quote, parameters, end, depth: 1 });
    quote = null;
    parameters = 0;
  }
  for (const line of text.split("\n")) {
    if (heredoc) {
      if (line.replace(/^\t+/, "").replace(/\r$/, "") === heredoc) heredoc = null;
    } else if (!line.startsWith("#!")) {
      for (let i = 0; i < line.length; i++) {
        const char = line[i];
        if (char === "\\" && quote !== "'") { i++; continue; }
        if (quote) {
          if (quote === '"' && line.startsWith("$(", i)) { command(")"); i++; }
          else if (quote === '"' && char === "`") command("`");
          else if (char === quote) quote = null;
          continue;
        }
        const active = commands.at(-1);
        if (active && char === active.end && (active.end !== ")" || --active.depth === 0)) {
          ({ quote, parameters } = commands.pop());
          continue;
        }
        if (line.startsWith("$(", i)) { command(")"); i++; continue; }
        if (char === "`") { command("`"); continue; }
        if (line.startsWith("${", i)) { parameters++; i++; continue; }
        if (parameters && char === "}") { parameters--; continue; }
        if (parameters) continue;
        if (active?.end === ")" && char === "(") active.depth++;
        if (char === '"' || char === "'") { quote = char; continue; }
        const doc = line.slice(i).match(/^<<-?\s*(['"]?)([A-Za-z_][A-Za-z_0-9]*)\1/);
        if (doc) { heredoc = doc[2]; i += doc[0].length - 1; continue; }
        if (char === "#" && (i === 0 || /[\s;|&()]/.test(line[i - 1]))) {
          spans.push([offset + i, offset + line.replace(/\r$/, "").length]); break;
        }
      }
    }
    offset += line.length + 1;
  }
  return spans;
}

function sqlComments(text) {
  const spans = [];
  let i = 0;
  while (i < text.length) {
    if (text.startsWith("--", i)) {
      const start = i, end = text.indexOf("\n", i);
      i = end < 0 ? text.length : end;
      spans.push([start, i]);
    } else if (text.startsWith("/*", i)) {
      const start = i, end = text.indexOf("*/", i + 2);
      if (end < 0) throw new Error("Unclosed SQL comment");
      i = end + 2; spans.push([start, i]);
    } else if (["'", '"', "`", "["].includes(text[i])) {
      const quote = text[i++] === "[" ? "]" : text[i - 1];
      while (i < text.length) {
        if (text[i++] === quote) {
          if (text[i] === quote) i++; else break;
        }
      }
    } else i++;
  }
  return spans;
}

export function comments(text, file) {
  switch (extname(file)) {
    case ".rs": return cComments(text, true);
    case ".kt": case ".kts": return cComments(text, false, true);
    case ".java": case ".gradle": return cComments(text, false);
    case ".ts": case ".tsx": case ".js": case ".jsx": case ".mjs": return jsComments(text, file);
    case ".vue": return vueComments(text, file);
    case ".css": return cssComments(text);
    case ".html": return htmlComments(text);
    case ".xml": return [...text.matchAll(/<!--[\s\S]*?-->|<!\[CDATA\[[\s\S]*?\]\]>/g)].filter(m => m[0].startsWith("<!--")).map(m => [m.index, m.index + m[0].length]);
    case ".sql": return sqlComments(text);
    case ".sh": return shellComments(text);
    case ".pro": return shellComments(text);
    case ".ps1": {
      const command = String.raw`$tokens = $null; $errors = $null; [System.Management.Automation.Language.Parser]::ParseFile($env:ZEDONE_COMMENT_SOURCE, [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count) { throw ($errors | Out-String) }; $spans = @($tokens | Where-Object { $_.Kind -eq 'Comment' -and $_.Text -notmatch '^#Requires\b' } | ForEach-Object { ,@($_.Extent.StartOffset, $_.Extent.EndOffset) }); ConvertTo-Json -Compress -InputObject $spans`;
      const directory = mkdtempSync(resolve(tmpdir(), "zedone-comment-source-"));
      try {
        const source = resolve(directory, "source.ps1");
        writeFileSync(source, text);
        return JSON.parse(execFileSync("pwsh", ["-NoProfile", "-NonInteractive", "-Command", command], { env: { ...process.env, ZEDONE_COMMENT_SOURCE: source }, encoding: "utf8" }));
      } finally { rmSync(directory, { recursive: true, force: true }); }
    }
    default: throw new Error(`Unsupported source type: ${file}`);
  }
}

export function withoutComments(text, file) {
  const original = text.split("\n");
  const spans = comments(text, file).sort((a, b) => b[0] - a[0]);
  for (const [start, end] of spans) text = text.slice(0, start) + text.slice(start, end).replace(/[^\r\n]/g, " ") + text.slice(end);
  return text.split("\n").filter((line, index) => !original[index].trim() || line.trim()).map(line => line.replace(/[ \t]+(?=\r?$)/, "")).join("\n");
}

export function checkSources(root = repository) {
  const errors = [];
  const files = sources(root);
  for (const file of files) {
    try {
      const text = readFileSync(resolve(root, file), "utf8");
      for (const [start] of comments(text, file)) errors.push(`${file}:${text.slice(0, start).split("\n").length}: comments are forbidden in project sources`);
    } catch (error) { errors.push(`${file}: cannot check source: ${error.message}`); }
  }
  if (errors.length) throw new Error(errors.join("\n"));
  return files;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    if (process.argv.length > 3 || process.argv[2] && process.argv[2] !== "--cargo") throw new Error("Usage: bun scripts/source-comments.mjs [--cargo]");
    const files = checkSources();
    if (process.argv[2] === "--cargo") {
      const manifest = process.env.CARGO_MANIFEST_DIR;
      if (!manifest) throw new Error("--cargo requires CARGO_MANIFEST_DIR");
      for (const path of [...policy.paths, "scripts/comment-policy.json"]) console.log(`cargo:rerun-if-changed=${relative(manifest, resolve(repository, path))}`);
    } else console.log(`Comment policy passed (${files.length} source files).`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
