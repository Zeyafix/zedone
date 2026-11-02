import assert from "node:assert/strict";
import { waitForAndroidPixelBaseline } from "./measure-android-pixels.mjs";

const expected = Buffer.from("all titles"), stale = Buffer.from("one title");
function fixture(frames, { timeout = 100, captureMs = 1, error, repeat = false } = {}) {
  let time = 0, index = 0;
  const evidence = {}, sources = [];
  return {
    evidence, sources,
    run: () => waitForAndroidPixelBaseline({
      expected, evidence, timeout, now: () => time, wait: async ms => { time += ms; },
      capture: async viaAdb => {
        sources.push(viaAdb);
        time += captureMs;
        if (error) throw error;
        return { data: frames[repeat ? index++ % frames.length : Math.min(index++, frames.length - 1)] };
      },
    }),
  };
}

// Stable old OS image after a ready DOM: wait for the known all-results image.
const delayed = fixture([stale, stale, stale, expected, expected, expected]);
await delayed.run();
assert.equal(delayed.evidence.status, "ready");
assert.deepEqual(delayed.evidence.attempts.map(a => a.captures.map(c => c.matches)), [[false, false, false], [true, true, true]]);
assert.deepEqual(delayed.sources, [true, true, false, true, true, false]);

// A transient match, or native readback lagging ADB, cannot arm the series.
for (const frames of [[expected, stale, expected], [expected, expected, stale]]) {
  const transient = fixture([...frames, expected, expected, expected]);
  await transient.run();
  assert.equal(transient.evidence.attempts.length, 2);
  assert.equal(transient.evidence.status, "ready");
}

for (const frames of [[stale], [expected, stale, expected]]) {
  const stuck = fixture(frames, { repeat: true });
  await assert.rejects(stuck.run, /baseline did not settle/);
  assert.equal(stuck.evidence.status, "failed");
  assert.ok(stuck.evidence.attempts.length > 0);
  assert.ok(stuck.evidence.elapsed_ms >= 100);
}
// A matching but late capture is still a preparation timeout.
const late = fixture([expected], { captureMs: 40 });
await assert.rejects(late.run, /baseline did not settle/);
assert.equal(late.evidence.status, "failed");
const broken = fixture([], { error: new Error("capture unavailable") });
await assert.rejects(broken.run, /capture unavailable/);
assert.equal(broken.evidence.status, "failed");
assert.match(broken.evidence.error, /capture unavailable/);
console.log("Android search preparation: stale/transient/divergent/late OS images and capture failure rejected; stable expected ADB/observer baseline accepted.");
