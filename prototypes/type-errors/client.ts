import { client } from "../client/http";
import { echo } from "../client/echo";
import { storage } from "../client/storage";
import { qr } from "../client/qr";

function rejectedCalls() {
  void qr({ text: 42 });
  void qr({ text: "synthetic", scale: 4 });
  void storage({ action: "move_card", card: 1, column: 0, position: 0 });
  void storage({ action: "move_column", expected: {}, column: 0, position: "1" });
  void storage({ action: "seed", space: "C" });
  void storage({ action: "read", space: "A" });
  void echo({ text: 42 });
  void echo();
  void client.POST("/unknown", { body: { text: "" } });
  void client.POST("/prototype/echo", { body: { text: 42 } });
  void client.POST("/prototype/echo", {});
}
void rejectedCalls;
