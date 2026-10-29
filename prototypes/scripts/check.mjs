import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "../scripts/environment-versions.json")).json();
const mode = Bun.argv[2];
if (Bun.argv.length > 3 || (mode && !["--write-openapi", "--check-openapi", "--write-types", "--check-types"].includes(mode))) {
  console.error("Expected no arguments, --write-openapi, --check-openapi, --write-types or --check-types.");
  process.exit(1);
}

if (Bun.version !== versions.bun) {
  console.error(`Expected Bun ${versions.bun}; found ${Bun.version}.`);
  process.exit(1);
}

// Pin the build output to this workspace even if the caller has a global override.
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const checks = [
  ["fmt", "--all", "--", "--check"],
  ["clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"],
  ["test", "--workspace", "--locked", "--offline"],
  ["build", "--workspace", "--release", "--locked", "--offline"],
];

console.log(`Runtime: Bun ${Bun.version}.`);

function run(cmd, capture = false) {
  console.log(`> ${cmd.join(" ")}`);
  const result = Bun.spawnSync(cmd, {
    cwd: root,
    env,
    stdin: "ignore",
    stdout: capture ? "pipe" : "inherit",
    stderr: "inherit",
  });
  if (result.exitCode !== 0 || result.signalCode) {
    console.error(`Prototype check failed: ${cmd.join(" ")}.`);
    process.exit(result.exitCode || 1);
  }
  return result.stdout;
}

const cargo = (args, capture = false) => run(["cargo", `+${versions.rust}`, ...args], capture);

if (!mode) {
  for (const args of checks) cargo(args);
}

// Capture first: compilation or generation failure must leave the artifact intact.
const generated = cargo([
  "run", "--quiet", "--locked", "--offline", "--bin", "generate-openapi",
], true);
const artifact = Bun.file(resolve(root, "openapi.json"));
if (mode === "--write-openapi") {
  await Bun.write(artifact, generated);
  console.log("Generated openapi.json from Rust; review its diff.");
} else {
  if (!(await artifact.exists()) || !generated.equals(Buffer.from(await artifact.arrayBuffer()))) {
    console.error("openapi.json is missing or stale. Run bun run generate:openapi and review the diff.");
    process.exit(1);
  }
  console.log("OpenAPI matches Rust.");
}

if (!mode || mode.endsWith("types")) {
  // Import the generator API in this Bun process: no CLI shebang/Node fallback.
  const { default: openapiTS, astToString } = await import("openapi-typescript");
  const types = astToString(await openapiTS(JSON.parse(generated.toString())));
  const command = JSON.parse(generated.toString()).paths["/prototype/echo"].post.operationId;
  const ipc = `// Generated from Rust/OpenAPI; do not edit.\nexport const echoCommand = ${JSON.stringify(command)} as const;\n`;
  const storageCommand = JSON.parse(generated.toString()).paths["/prototype/storage"].post.operationId;
  const commands = ipc + `export const storageCommand = ${JSON.stringify(storageCommand)} as const;\n`;
  const ipcFile = Bun.file(resolve(root, "client/ipc-command.ts"));
  const typesFile = Bun.file(resolve(root, "client/api.d.ts"));
  if (mode === "--write-types") {
    await Bun.write(typesFile, types);
    await Bun.write(ipcFile, commands);
    console.log("Generated client/api.d.ts and client/ipc-command.ts from OpenAPI; review their diff.");
  } else if (!(await typesFile.exists()) || await typesFile.text() !== types || !(await ipcFile.exists()) || await ipcFile.text() !== commands) {
    console.error("client/api.d.ts or client/ipc-command.ts is missing or stale. Run bun run generate:types and review the diff.");
    process.exit(1);
  }
}

if (!mode) {
  // vue-tsc patches the CommonJS loader through fs.readFileSync; Bun bypasses it.
  const nodeVersion = run(["node", "--version"], true).toString().trim();
  if (nodeVersion !== `v${versions.node}`) {
    console.error(`vue-tsc requires pinned Node ${versions.node}; found ${nodeVersion}.`);
    process.exit(1);
  }
  run(["node", "node_modules/vue-tsc/bin/vue-tsc.js", "--noEmit"]);
  // Explicit Bun entry point bypasses Vite's Node shebang.
  run([process.execPath, "node_modules/vite/bin/vite.js", "build"]);
  run([process.execPath, "scripts/check-http.mjs"]);
}
