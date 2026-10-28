import { client } from "./http";
import { echo } from "./echo";

// Compiled, never executed: these must remain errors after every generation.
function rejectedCalls() {
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
