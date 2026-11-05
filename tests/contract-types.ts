// Compile-only consumers of the generated product contract; no test operation
// or transport is registered by these fixtures.
import type { components } from "../api/schema";

type Request = components["schemas"]["CommandRequest_Value"];
type Response = components["schemas"]["CommandResponse_Value"];
type ErrorCode = components["schemas"]["ErrorCode"];

const id: Request["id"] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const request = { id, operation: "fixture.only", parameters: { title: "Ёж 😀" } } satisfies Request;
// @ts-expect-error A short ID cannot be sent as a complete intention.
const shortId: Request["id"] = [1];
// The generator caps large tuple expansion; versions remain number[] in TS.
// Byte ranges and all lengths are validated by Rust, including 32-byte versions.
// @ts-expect-error Authentication and permissions are not request parameters.
const forged = { ...request, subject: "administrator" } satisfies Request;
// @ts-expect-error A success needs the original result and replay indicator.
const incomplete: Response = { status: "ok" };
// @ts-expect-error Unknown outcome is an error, never an alternative success.
const falseSuccess: Response = { status: "outcome_unknown", result: null, replayed: false };

function consume(response: Response): unknown {
  if (response.status === "ok") {
    const replayed: boolean = response.replayed;
    // @ts-expect-error The envelope does not invent a product result type.
    const title = response.result.title;
    return { result: response.result, replayed };
  }
  const code: ErrorCode = response.error.code;
  switch (code) {
    case "invalid_request": case "access_denied": case "not_found":
    case "stale_state": case "command_mismatch": case "unavailable":
    case "outcome_unknown": return code;
    default: {
      const exhaustive: never = code;
      return exhaustive;
    }
  }
}

consume({ status: "ok", result: null, replayed: true });
consume({ status: "error", error: { code: "outcome_unknown" } });
