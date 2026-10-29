import { client } from "./http";
import { echo } from "./echo";
import { storage } from "./storage";

// Compiled, never executed: these must remain errors after every generation.
function rejectedCalls() {
  // @ts-expect-error A move requires the original Rust-derived snapshot.
  void storage({ action: "move_card", card: 1, column: 0, position: 0 });
  // @ts-expect-error Positions are numbers in both HTTP and IPC.
  void storage({ action: "move_column", expected: {}, column: 0, position: "1" });
  // @ts-expect-error The space enum comes from Rust, not arbitrary database paths.
  void storage({ action: "seed", space: "C" });
  // @ts-expect-error A read requires the literal search query.
  void storage({ action: "read", space: "A" });
  // @ts-expect-error IPC and HTTP accept the same Rust-derived request.
  void echo({ text: 42 });
  // @ts-expect-error IPC also requires the body.
  void echo();
  // @ts-expect-error Only Rust-described paths are callable.
  void client.POST("/unknown", { body: { text: "" } });
  // @ts-expect-error The required string cannot be replaced with a number.
  void client.POST("/prototype/echo", { body: { text: 42 } });
  // @ts-expect-error The request body is required.
  void client.POST("/prototype/echo", {});
}
void rejectedCalls;
