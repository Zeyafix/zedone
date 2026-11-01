import assert from "node:assert/strict";

// A constant reply measures the existing instrumentation/WebView round trip,
// never completion of an application command or presentation of a frame.
export async function measureNoopEvaluation({ evaluate, setTraceLabel, evidence, now = () => performance.now(), wait = ms => new Promise(resolve => setTimeout(resolve, ms)) }) {
  const script = () => null;
  Object.assign(evidence, { status: "running", script: script.toString(), expected_result: null, inter_request_pause_ms: 50, warmups: [], samples: [] });
  try {
    for (let repeat = -5; repeat < 30; repeat++) {
      await wait(evidence.inter_request_pause_ms);
      const sample = { repeat, trace_label: `zedone:noop-${repeat < 0 ? "warmup" + -repeat : repeat}`, ok: false };
      (repeat < 0 ? evidence.warmups : evidence.samples).push(sample);
      setTraceLabel(sample.trace_label);
      const start = now();
      try {
        sample.result = await evaluate(script);
        sample.evaluation_request_ms = now() - start;
        assert.equal(sample.result, null, "Unexpected no-op evaluation reply; no retry allowed");
        sample.ok = true;
      } catch (error) {
        sample.evaluation_request_ms ??= now() - start;
        sample.error = String(error);
        throw error;
      } finally { setTraceLabel(undefined); }
    }
    evidence.status = "completed";
  } catch (error) { evidence.status = "failed"; evidence.error = String(error); throw error; }
}
