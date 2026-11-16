import { test, expect } from "bun:test";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, dirname, sep } from "node:path";
import { comments, checkSources, withoutComments } from "../scripts/source-comments.mjs";

const cases = [
  ["a.rs", 'fn f() {} // trailing\n/// doc\n/* outer /* inner */ end */', 3],
  ["a.rs", 'fn f<\'a>(s: &\'a str) { let a = r###"/* text */ // text"###; let b = b"//"; let c = \'/\'; }', 0],
  ["a.ts", 'const a = "https://host/*"; const r = /https?:\\/\\//; const t = `// ${"/*"}`;', 0],
  ["a.ts", 'const t = `${ /* expression */ 1}`; /** doc */ const a = 1; // end', 3],
  ["a.d.ts", '/** API */ export declare const v: number;', 1],
  ["a.vue", '<!-- top --><template><!-- template --><p>{{ /* expression */ value }}</p></template><script setup lang="ts">const url = "https://host"; // script\n</script><style>p { /* css */ color:red }</style>', 5],
  ["a.vue", '<template><button @click="/* event */ submit()" v-if="/* condition */ ready">{{ "// text" }}</button></template>', 2],
  ["a.vue", '<template><div v-for="(item, index) in /* list */ items">{{item}}</div></template>', 1],
  ["a.html", '<!-- comment --><script>const a = "<!-- text -->"; /* js */</script><style>/* css */</style>', 3],
  ["a.xml", '<root><![CDATA[<!-- text -->]]><!-- comment --></root>', 1],
  ["a.css", 'a { content: "/* text */"; background: url("https://host"); /* comment */ }', 1],
  ["a.kt", 'val a = "//"; val b = """/* text */"""; /* a /* b */ c */ // end', 2],
  ["a.kt", 'val a = "${ /* expression */ 1 }"; val b = """${ /* nested */ "// text" }"""', 2],
  ["a.java", 'String s = "https://host"; /* comment */', 1],
  ["a.sql", "select '-- text', '/* text */', 'it''s -- text'; -- comment\n/* block */", 2],
  ["a.sh", '#!/bin/sh\na="https://host/#text"\n# comment\necho hi # end\ncat <<\'TEXT\'\n# data\nTEXT\n', 2],
  ["a.sh", 'a="$(printf x # command\n)"\nb=`printf y # legacy\n`\nc=${missing:- # data}\n', 2],
  ["a.ps1", '#Requires -Version 7.0\n$a = "# text"\n<# block #>\nWrite-Output $a # comment', 2],
  ["a.pro", '# comment\n-keep class app.Main # end', 2],
  ["a.pro", '-injars "libs/#data.jar"', 0],
];

for (const [file, text, count] of cases) test(`distinguishes comments and data in ${file}: ${text.slice(0, 35)}`, () => {
  expect(comments(text, file)).toHaveLength(count);
  expect(comments(withoutComments(text, file), file)).toHaveLength(0);
});

test("rejects comments in new sources and preserves the third-party boundary", () => {
  const root = mkdtempSync(resolve(tmpdir(), "zedone-comment-policy-"));
  try {
    mkdirSync(resolve(root, "client"), { recursive: true });
    writeFileSync(resolve(root, "client/new.ts"), 'export const url = "https://host";');
    expect(checkSources(root)).toContain("client/new.ts");
    writeFileSync(resolve(root, "client/new.ts"), "// forbidden\nexport const value = 1;");
    expect(() => checkSources(root)).toThrow("client/new.ts:1");
    writeFileSync(resolve(root, "client/new.ts"), "export const value = 1;");
    const generated = "ui/src-tauri/gen/android/app/src/main/java/app/zedone/local/generated";
    mkdirSync(resolve(root, generated), { recursive: true });
    writeFileSync(resolve(root, generated, "Upstream.kt"), "// upstream license");
    mkdirSync(resolve(root, "docs"));
    writeFileSync(resolve(root, "docs/example.ts"), "// documentation example");
    expect(checkSources(root)).toEqual(["client/new.ts"]);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

const impact = Bun.YAML.parse(readFileSync(new URL("./fixtures/source-comments-impact.yaml", import.meta.url), "utf8"));
for (const scenario of impact.cases) test(`PR impact: ${scenario.id}`, () => {
  const root = mkdtempSync(resolve(tmpdir(), "zedone-comment-impact-"));
  try {
    const file = resolve(root, scenario.file);
    expect(file.startsWith(root + sep)).toBe(true);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, scenario.source);
    if (scenario.after === "rejected") expect(() => checkSources(root)).toThrow(scenario.diagnostic);
    else {
      expect(scenario.after).toBe("accepted");
      expect(() => checkSources(root)).not.toThrow();
    }
    if (scenario.recovery_source !== undefined) {
      writeFileSync(file, scenario.recovery_source);
      expect(() => checkSources(root)).not.toThrow();
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
});
