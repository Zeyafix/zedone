import assert from "node:assert/strict";
import { measureNoopEvaluation } from "./android-noop-evaluation.mjs";

function fixture({ wrongAt, errorAt } = {}) {
  let time = 0, calls = 0;
  const evidence = {}, labels = [], pauses = [];
  return { evidence, labels, pauses, get calls() { return calls; }, run: () => measureNoopEvaluation({
    evidence, now: () => time,
    wait: async ms => { time += ms; pauses.push(ms); },
    setTraceLabel: label => labels.push(label),
    evaluate: async script => {
      const index = calls++; time += 12;
      assert.equal(script(), null, "Sampler must dispatch only the constant no-op");
      if (index === errorAt) throw new Error("evaluation unavailable");
      return index === wrongAt ? false : script();
    },
  }) };
}

const stable = fixture(); await stable.run();
assert.equal(stable.evidence.status, "completed");
assert.equal(stable.evidence.warmups.length, 5);assert.equal(stable.evidence.samples.length, 30);
assert.equal(stable.calls, 35);assert.deepEqual(stable.pauses, Array(35).fill(50));
assert.equal(new Set(stable.labels.filter(Boolean)).size, 35);
assert.equal(stable.labels.at(-1), undefined);
for (const [index, sample] of [...stable.evidence.warmups, ...stable.evidence.samples].entries()) {
  assert.equal(sample.repeat, index - 5);
  assert.equal(sample.evaluation_request_ms, 12, "Pacing must remain outside the request clock");
  assert.equal(sample.result, null);assert.equal(sample.ok, true);
}
for (const options of [{ wrongAt: 0 }, { wrongAt: 5 }, { errorAt: 0 }, { errorAt: 5 }]) {
  const broken = fixture(options);await assert.rejects(broken.run, /Unexpected no-op|evaluation unavailable/);
  assert.equal(broken.evidence.status, "failed");
  const index = options.wrongAt ?? options.errorAt;
  assert.equal(broken.calls, index + 1, "Failed evaluation must not be retried or omitted");
  const sample = [...broken.evidence.warmups, ...broken.evidence.samples].at(-1);
  assert.equal(sample.ok, false);assert.equal(sample.evaluation_request_ms, 12);assert.ok(sample.error);
  assert.equal(broken.labels.at(-1), undefined);
}
for (const args of [
  ["--pixels", "--release", "--trace-noop"],
  ["--pixels", "--trace-noop", "--deflate-pixels"],
  ["--pixels", "--release", "--trace-moves", "--trace-noop", "--deflate-pixels"],
  ["--pixels", "--release", "--trace-static", "--trace-noop", "--deflate-pixels"],
]) {
  const result = Bun.spawnSync([process.execPath, `${import.meta.dir}/measure-android.mjs`, ...args], { env: { ...process.env, ANDROID_HOME: "", ANDROID_SERIAL: "" } });
  assert.notEqual(result.exitCode, 0);assert.match(result.stderr.toString(), /--trace-noop requires/);
}
console.log("No-op evaluation: fixed constant reply, 5+30, separate request clock, no retry on warmup/sample error or wrong reply, label cleanup and invalid CLI combinations passed.");
