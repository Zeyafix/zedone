import assert from "node:assert/strict";
import { measureStaticPixels } from "./android-static-pixels.mjs";

const expected = Buffer.from([1, 2, 3, 255]);
function fixture({ mismatchAt, captureError, saveError } = {}) {
  let time = 0, calls = 0;
  const evidence = {}, labels = [], saved = [], pauses = [];
  return { evidence, labels, saved, pauses, get calls() { return calls; }, run: () => measureStaticPixels({
    evidence, expected, now: () => time,
    wait: async ms => { pauses.push(ms); time += ms; },
    setTraceLabel: label => labels.push(label),
    capture: async () => {
      const index = calls++; time += 12;
      if (captureError) throw new Error("readback unavailable");
      return { data: index === mismatchAt ? Buffer.from([1, 2, 4, 255]) : expected };
    },
    saveImage: async (name, image) => {
      saved.push({ name, image }); time += 100;
      if (saveError) throw new Error("file output failed");
      return `${name}.png`;
    },
  }) };
}

const stable = fixture();
await stable.run();
assert.equal(stable.evidence.status, "completed");
assert.equal(stable.evidence.warmups.length, 5);
assert.equal(stable.evidence.samples.length, 30);
assert.equal(stable.calls, 35);
assert.equal(stable.saved.length, 35);
assert.deepEqual(stable.pauses, Array(35).fill(50));
for (const sample of [...stable.evidence.warmups, ...stable.evidence.samples]) {
  assert.equal(sample.capture_request_ms, 12, "Pacing and PNG file output must not enter the request metric");
  assert.equal(sample.ok, true); assert.equal(sample.matched, true);
  assert.equal(sample.sha256, stable.evidence.expected_sha256);
}
assert.equal(new Set(stable.labels.filter(Boolean)).size, 35);
assert.equal(stable.labels.at(-1), undefined);

for (const mismatchAt of [0, 5]) {
  const changed = fixture({ mismatchAt });
  await assert.rejects(changed.run, /Static OS image changed/);
  assert.equal(changed.calls, mismatchAt + 1, "A changed static frame must not be retried or omitted");
  const failed = [...changed.evidence.warmups, ...changed.evidence.samples].at(-1);
  assert.equal(changed.evidence.status, "failed");
  assert.equal(failed.ok, false); assert.equal(failed.matched, false);
  assert.ok(failed.file); assert.notEqual(failed.sha256, changed.evidence.expected_sha256);
  assert.equal(changed.labels.at(-1), undefined);
}
for (const options of [{ captureError: true }, { saveError: true }]) {
  const broken = fixture(options);
  await assert.rejects(broken.run, /unavailable|output failed/);
  assert.equal(broken.evidence.status, "failed");
  assert.equal(broken.calls, 1);
  assert.equal(broken.evidence.warmups[0].capture_request_ms, 12);
  assert.equal(broken.labels.at(-1), undefined);
}
// Invalid combinations fail before SDK/device lookup, APK reads or mutation.
for (const args of [
  ["--pixels", "--release", "--trace-static"],
  ["--pixels", "--trace-static", "--deflate-pixels"],
  ["--pixels", "--release", "--trace-moves", "--trace-static", "--deflate-pixels"],
]) {
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/measure-android.mjs`, ...args], { env: { ...process.env, ANDROID_HOME: "", ANDROID_SERIAL: "" } });
  assert.notEqual(result.exitCode, 0);
  assert.match(result.stderr.toString(), /--trace-static requires/);
}
console.log("Static OS observer: 5+30 exact frames, separate request clock, no retry on warmup/sample mismatch, capture/output failures, label cleanup and invalid CLI combinations passed.");
