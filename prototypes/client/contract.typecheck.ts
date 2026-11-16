import { client } from "./http";
import { echo } from "./echo";
import { storage } from "./storage";
import { qr } from "./qr";

function acceptedCalls() {
  void qr({ text: "synthetic" });
  void storage({ action: "read", space: "A", query: "" });
  void echo({ text: "synthetic" });
  void client.POST("/prototype/echo", { body: { text: "synthetic" } });
}
void acceptedCalls;
