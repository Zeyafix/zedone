import { createHash } from "node:crypto";

// One readback per sample. An already visible image is the intended static
// input, never an observed completion of an application action.
export async function measureStaticPixels({ capture, expected, setTraceLabel, saveImage, evidence, now = () => performance.now(), wait = ms => new Promise(resolve => setTimeout(resolve, ms)) }) {
  Object.assign(evidence, { status: "running", expected_sha256: createHash("sha256").update(expected).digest("hex"), inter_capture_pause_ms: 50, warmups: [], samples: [] });
  try {
    for (let repeat = -5; repeat < 30; repeat++) {
      await wait(evidence.inter_capture_pause_ms);
      const sample = { repeat, trace_label: `zedone:static-${repeat < 0 ? "warmup" + -repeat : repeat}`, ok: false };
      (repeat < 0 ? evidence.warmups : evidence.samples).push(sample);
      setTraceLabel(sample.trace_label);
      const start = now();
      try {
        const image = await capture();
        sample.capture_request_ms = now() - start;
        sample.matched = image.data.equals(expected);
        sample.sha256 = createHash("sha256").update(image.data).digest("hex");
        // File output and hashing do not belong to capture_request_ms.
        sample.file = await saveImage(`static-${repeat < 0 ? "warmup" + -repeat : repeat}`, image);
        if (!sample.matched) throw new Error("Static OS image changed; no retry allowed");
        sample.ok = true;
      } catch (error) {
        sample.capture_request_ms ??= now() - start;
        sample.error = String(error);
        throw error;
      }
      finally { setTraceLabel(undefined); }
    }
    evidence.status = "completed";
  } catch (error) { evidence.status = "failed"; evidence.error = String(error); throw error; }
}
