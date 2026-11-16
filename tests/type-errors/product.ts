import type { components } from "../../api/schema";
import { prepare, type IntentionStore } from "../../client/commands";

function typedCommands(store: IntentionStore) {
  void prepare("2".repeat(32), "access.inspect", { subject: "1".repeat(32) }, store);
  void prepare("2".repeat(32), "access.list", { after: null }, store);
  void prepare("2".repeat(32), "access.issue", { subject: "1".repeat(32), verifier: Array(32).fill(1), scope: { kind: "spaces", spaces: { ["3".repeat(32)]: "read" } }, expires_at_unix_ms: null }, store);
  void prepare("2".repeat(32), "access.issue", { subject: "1".repeat(32), verifier: Array(32).fill(1) }, store);
  void prepare("2".repeat(32), "access.replace", { subject: "1".repeat(32), verifier: Array(32).fill(1), scope: { kind: "administration" }, expires_at_unix_ms: null }, store);
  void prepare("2".repeat(32), "access.revoke", { subject: "1".repeat(32), expected: Array(32).fill(1) }, store);
  void prepare("2".repeat(32), "access.revoke", { subject: "1".repeat(32) }, store);
  void prepare("2".repeat(32), "access.bootstrap", {}, store);
  void prepare("2".repeat(32), "space.create", { space: "3".repeat(32), name: "空間" }, store);
  void prepare("2".repeat(32), "space.list", { include_trashed: false }, store);
  void prepare("2".repeat(32), "space.create", { space: "3".repeat(32), name: "x", subject: "2".repeat(32) }, store);
  void prepare("2".repeat(32), "space.rename", { space: "3".repeat(32), name: "x" }, store);
}
void typedCommands;

const combinedScope: components["schemas"]["AccessScope"] = {
  kind: "administration_and_spaces", spaces: { a: "read_write", b: "read" },
};
void combinedScope;

type Request = components["schemas"]["CommandRequest_Value"];
type Response = components["schemas"]["CommandResponse_Value"];
type ErrorCode = components["schemas"]["ErrorCode"];

const id: Request["id"] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const request = { id, operation: "fixture.only", parameters: { title: "Ёж 😀" } } satisfies Request;
const shortId: Request["id"] = [1];
const forged = { ...request, subject: "administrator" } satisfies Request;
const incomplete: Response = { status: "ok" };
const falseSuccess: Response = { status: "outcome_unknown", result: null, replayed: false };

function consume(response: Response): unknown {
  if (response.status === "ok") {
    const replayed: boolean = response.replayed;
    const title = response.result.title;
    return { result: response.result, replayed };
  }
  const code: ErrorCode = response.error.code;
  switch (code) {
    case "invalid_request": case "access_denied": case "not_found":
    case "stale_state": case "conflict": case "command_mismatch": case "unavailable":
    case "outcome_unknown": return code;
    default: {
      const exhaustive: never = code;
      return exhaustive;
    }
  }
}

consume({ status: "ok", result: null, replayed: true });
consume({ status: "error", error: { code: "outcome_unknown" } });
