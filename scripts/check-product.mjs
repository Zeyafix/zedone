import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { cp, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, relative, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const versions = await Bun.file(resolve(root, "scripts/environment-versions.json")).json();
assert.equal(Bun.version, versions.bun, "Use pinned Bun");
const mode = Bun.argv[2];
assert.ok(Bun.argv.length <= 3 && [undefined, "--write-openapi", "--check-openapi", "--write-types", "--check-types", "--android"].includes(mode));
const env = { ...process.env, CARGO_TARGET_DIR: resolve(root, "target") };
const suffix = process.platform === "win32" ? ".exe" : "";

function run(command, { cwd = root, extraEnv = {}, capture = false, failure = false } = {}) {
  console.log(`> ${command.join(" ")}`);
  const result = Bun.spawnSync(command, {
    cwd, env: { ...env, ...extraEnv }, stdin: "ignore",
    stdout: capture || failure ? "pipe" : "inherit",
    stderr: failure ? "pipe" : "inherit",
  });
  assert.ok(!result.signalCode, `Command interrupted: ${command[0]}`);
  if (!failure && result.exitCode !== 0 && capture) process.stderr.write(result.stdout);
  if (!failure) assert.equal(result.exitCode, 0, `Command failed: ${command[0]}`);
  return result;
}
const cargo = args => ["cargo", `+${versions.rust}`, ...args];
const locked = ["--locked", "--offline"];
const metadata = JSON.parse(run(cargo(["metadata", "--no-deps", "--format-version", "1", ...locked]), { capture: true }).stdout);
const packages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
const manifest = Bun.TOML.parse(await Bun.file(resolve(root, "Cargo.toml")).text());
assert.equal(manifest.workspace.lints.rust.unsafe_code, "forbid");
for (const pkg of packages) {
  assert.ok(relative(root, pkg.manifest_path).replaceAll("\\", "/").startsWith("crates/"));
  const member = Bun.TOML.parse(await Bun.file(pkg.manifest_path).text());
  assert.equal(member.lints?.workspace, true, `${pkg.name} must inherit workspace lints`);
  assert.ok(!pkg.dependencies.some(dep => dep.name.startsWith("zedone-prototype-")));
  if (pkg.name === "zedone-core") {
    assert.ok(pkg.dependencies.every(dep => ["serde", "serde_json", "utoipa"].includes(dep.name)), "Review core dependency boundaries before extending them");
  }
}

// Inject only into a disposable copy, independently for every Cargo target.
// The package-level forbid must also reject a local attempt to allow unsafe.
async function checkUnsafe(targetArgs = [], extraEnv = {}) {
  const temporary = await mkdtemp(resolve(tmpdir(), "zedone-product-unsafe-"));
  try {
    for (const name of ["Cargo.toml", "Cargo.lock", "crates"]) await cp(resolve(root, name), resolve(temporary, name), { recursive: true });
    for (const pkg of packages) {
      for (const target of pkg.targets) {
        const file = resolve(temporary, relative(root, target.src_path));
        const original = await Bun.file(file).text();
        assert.ok(target.kind.some(kind => ["lib", "bin", "test"].includes(kind)), `Add unsafe coverage for ${target.kind}`);
        const selector = target.kind.includes("lib") ? ["--lib"] : [target.kind.includes("bin") ? "--bin" : "--test", target.name];
        try {
          await Bun.write(file, `${original}\n#[allow(unsafe_code)]\nunsafe fn p03_01_forbidden() {}\n`);
          const result = run(cargo(["check", "-p", pkg.name, ...selector, "--all-features", ...targetArgs, ...locked, "--message-format=json"]), { cwd: temporary, extraEnv, failure: true });
          const messages = result.stdout.toString().split("\n").filter(Boolean).map(line => JSON.parse(line));
          assert.notEqual(result.exitCode, 0, `Unsafe compiled in ${target.name}`);
          assert.ok(messages.some(item => item.reason === "compiler-message" && item.message.level === "error" && item.message.code?.code === "unsafe_code"), `Expected unsafe_code diagnostic in ${target.name}`);
          console.log(`unsafe rejected: ${pkg.name}/${target.name} ${targetArgs.join(" ")}`);
        } finally {
          await Bun.write(file, original);
        }
      }
    }
  } finally {
    assert.equal(dirname(temporary), resolve(tmpdir()));
    assert.ok(basename(temporary).startsWith("zedone-product-unsafe-"));
    await rm(temporary, { recursive: true });
  }
}

if (!mode) {
  for (const args of [
    ["fmt", "--all", "--", "--check"],
    ["clippy", "--workspace", "--all-targets", "--all-features", ...locked, "--", "-D", "warnings"],
    ["test", "--workspace", "--all-features", ...locked],
    ["build", "--workspace", "--all-targets", "--all-features", "--release", ...locked],
  ]) run(cargo(args));
  await checkUnsafe();
}

const generated = run(cargo(["run", "--quiet", "--bin", "export-contract", ...locked]), { capture: true }).stdout;
const artifact = Bun.file(resolve(root, "api/openapi.json"));
if (mode === "--write-openapi") {
  await Bun.write(artifact, generated);
  console.log("Generated api/openapi.json from Rust.");
} else {
  assert.ok(await artifact.exists(), "Generate api/openapi.json first");
  assert.ok(generated.equals(Buffer.from(await artifact.arrayBuffer())), "api/openapi.json is stale; run bun scripts/check-product.mjs --write-openapi");
  console.log("Product OpenAPI matches Rust.");
}

if (mode !== "--write-openapi") {
  const { default: openapiTS, astToString } = await import("openapi-typescript");
  const types = astToString(await openapiTS(JSON.parse(generated.toString()), { arrayLength: true }));
  const typesFile = Bun.file(resolve(root, "api/schema.d.ts"));
  // Compile the Rust-derived response schema ahead of time: browser/IPC code
  // needs neither Ajv nor dynamic code evaluation, and has no hand-copied DTO.
  const { default: Ajv } = await import("ajv/dist/2020.js");
  const { default: standalone } = await import("ajv/dist/standalone/index.js");
  const schemas = JSON.parse(JSON.stringify(JSON.parse(generated.toString()).components.schemas)
    .replaceAll("#/components/schemas/", "#/$defs/"));
  const ajv = new Ajv({ strict: false, validateFormats: false, code: { source: true, esm: true, lines: true } });
  // Derive operation-specific validation and client types from concrete Rust
  // paths. A valid result for another operation cannot confirm an intention.
  const routes = Object.entries(JSON.parse(generated.toString()).paths).map(([path, item]) => ({
    operation: path.slice("/v1/commands/".length),
    request: item.post.requestBody.content["application/json"].schema.$ref.split("/").at(-1),
    response: item.post.responses["200"].content["application/json"].schema.$ref.split("/").at(-1),
  })).sort((a, b) => a.operation.localeCompare(b.operation));
  const responses = [...new Set(routes.map(route => route.response))];
  ajv.addSchema({ $id: "zedone.responses", $defs: schemas });
  const checks = Object.fromEntries(responses.map(name => [name, `zedone.responses#/$defs/${name}`]));
  const mapping = routes.map(route => `${JSON.stringify(route.operation)}: ${route.response}`).join(",\n");
  const validator = standalone(ajv, checks) + `\nconst responseValidators = Object.freeze({\n${mapping}\n});\nexport const operations = Object.freeze(Object.keys(responseValidators));\nexport default function validate(value, operation) {\n  if (operation !== undefined) return Object.hasOwn(responseValidators, operation) && responseValidators[operation](value);\n  return Object.values(responseValidators).some(check => check(value));\n}\n`;
  assert.ok(!validator.includes("require(") && !validator.includes("import "), "Validator must remain browser standalone");
  const validatorTypes = 'import type { components } from "./schema";\n'
    + `export type Operation = ${routes.map(route => JSON.stringify(route.operation)).join(" | ")};\n`
    + `export type Parameters = {\n${routes.map(route => `  ${JSON.stringify(route.operation)}: components["schemas"][${JSON.stringify(route.request)}]["parameters"];`).join("\n")}\n};\n`
    + `export type Response = ${responses.map(name => `components["schemas"][${JSON.stringify(name)}]`).join(" | ")};\n`
    + 'export const operations: readonly Operation[];\nexport default function validate(value: unknown, operation?: Operation): value is Response;\n';
  if (mode === "--write-types") {
    await Bun.write(typesFile, types);
    await Bun.write(resolve(root, "api/validate-response.js"), validator);
    await Bun.write(resolve(root, "api/validate-response.d.ts"), validatorTypes);
    console.log("Generated api/schema.d.ts from Rust/OpenAPI.");
  } else {
    assert.ok(await typesFile.exists() && await typesFile.text() === types,
      "api/schema.d.ts is missing or stale; run bun scripts/check-product.mjs --write-types");
    console.log("Product TypeScript matches Rust/OpenAPI.");
    assert.equal(await Bun.file(resolve(root, "api/validate-response.js")).text(), validator,
      "Response validator is stale; run --write-types");
    assert.equal(await Bun.file(resolve(root, "api/validate-response.d.ts")).text(), validatorTypes,
      "Response validator types are stale; run --write-types");
  }
  run([process.execPath, resolve(root, "node_modules/typescript/bin/tsc"), "-p", "tsconfig.contract.json"]);
  if (!mode || mode === "--android") run([process.execPath, "test", resolve(root, "tests/commands.test.ts")]);
}

if (mode === "--android") {
  const serial = env.ANDROID_SERIAL;
  assert.match(serial || "", /^emulator-\d+$/, "Explicit test emulator required");
  assert.ok(env.ANDROID_HOME && env.NDK_HOME, "ANDROID_HOME and NDK_HOME required");
  assert.ok((await Bun.file(resolve(env.NDK_HOME, "source.properties")).text()).includes(versions.android.ndk));
  const adb = resolve(env.ANDROID_HOME, `platform-tools/adb${suffix}`);
  const device = (...args) => run([adb, "-s", serial, ...args], { capture: true }).stdout.toString().replaceAll("\r\n", "\n");
  for (const [property, expected] of [["ro.kernel.qemu", "1"], ["ro.build.version.sdk", "33"], ["ro.product.cpu.abi", "x86_64"]]) {
    assert.equal(device("shell", "getprop", property).trim(), expected);
  }
  const host = process.platform === "win32" ? "windows-x86_64" : process.platform === "darwin" ? "darwin-x86_64" : "linux-x86_64";
  const tools = resolve(env.NDK_HOME, `toolchains/llvm/prebuilt/${host}/bin`);
  const linker = resolve(tools, `clang${suffix}`);
  for (const target of ["x86_64-linux-android", "aarch64-linux-android"]) {
    const prefix = `CARGO_TARGET_${target.replaceAll("-", "_").toUpperCase()}`;
    const lower = target.replaceAll("-", "_");
    const extraEnv = {
      [`CC_${lower}`]: linker,
      [`AR_${lower}`]: resolve(tools, `llvm-ar${suffix}`),
      [`CFLAGS_${lower}`]: `--target=${target}33`,
      [`${prefix}_LINKER`]: linker,
      [`${prefix}_RUSTFLAGS`]: `-C link-arg=--target=${target}33`,
    };
    run(cargo(["build", "--workspace", "--all-targets", "--all-features", "--release", "--target", target, ...locked]), { extraEnv });
    await checkUnsafe(["--target", target], extraEnv);
    if (target !== "x86_64-linux-android") continue;
    const result = run(cargo(["test", "--workspace", "--all-features", "--no-run", "--message-format=json", "--target", target, ...locked]), { extraEnv, capture: true });
    const tests = result.stdout.toString().split("\n").filter(Boolean).map(line => JSON.parse(line))
      .filter(item => item.reason === "compiler-artifact" && item.profile.test && item.executable);
    assert.ok(tests.length > 0, "Expected native test binaries");
    const remote = `/data/local/tmp/zedone-product-${randomUUID()}`;
    device("shell", "mkdir", remote);
    try {
      device("push", resolve(root, `target/${target}/release/zedone`), `${remote}/zedone`);
      device("shell", "chmod", "700", `${remote}/zedone`);
      for (const [index, test] of tests.entries()) {
        const executable = `${remote}/test-${index}`;
        device("push", test.executable, executable);
        device("shell", "chmod", "700", executable);
        // tempfile and crash-child fixtures must stay in our removable sandbox.
        const output = device("shell", `TMPDIR=${remote}`, `ZEDONE_TEST_CLI=${remote}/zedone`, executable);
        console.log(output);
        assert.ok(output.includes("test result: ok."));
      }
      device("push", resolve(root, `target/${target}/release/export-contract`), `${remote}/export-contract`);
      device("shell", "chmod", "700", `${remote}/export-contract`);
      assert.equal(device("shell", `${remote}/export-contract`), generated.toString(), "Android and host contracts differ");
    } finally {
      assert.match(remote, /^\/data\/local\/tmp\/zedone-product-[a-f0-9-]+$/);
      device("shell", "rm", "-r", remote);
    }
  }
  console.log("Android native tests/contract passed; both ABI builds and unsafe guards passed.");
}
