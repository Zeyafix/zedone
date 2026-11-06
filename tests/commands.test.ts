import { test, expect } from "bun:test";
import { CommandClient, prepare, newSpaceId, newColumnId, httpTransport, ipcTransport, webIntentionStore, type Response, type IntentionStore } from "../client/commands";
import validateResponse from "../api/validate-response.js";

function fixture() {
  const values = new Map<string, string>();
  const storage = {
    get length() { return values.size; },
    key(index: number) { return [...values.keys()][index] ?? null; },
    getItem(key: string) { return values.get(key) ?? null; },
    setItem(key: string, value: string) { values.set(key, value); },
    removeItem(key: string) { values.delete(key); },
  };
  return { values, storage, store: webIntentionStore(storage, "test") };
}
const parameters = { subject: "1".repeat(32) };
const success: Response = { status: "ok", replayed: true, result: {
  subject: parameters.subject, scope: { kind: "administration" }, expires_at_unix_ms: null, revoked: true, version: Array(32).fill(1),
} };

test("column intentions preserve their parent, ID and original version across restart and verify their own result", async () => {
  const {store, storage}=fixture();
  const space=newSpaceId(), column=newColumnId();
  expect(column).toMatch(/^[a-f0-9]{32}$/); expect(newColumnId()).not.toBe(column);
  const parameters={space,column,expected:Array(32).fill(7)};
  const id=await prepare("2".repeat(32),"column.trash",parameters,store);
  const saved=(await store.load(id))!.body;
  parameters.expected[0]=8;
  const restarted=webIntentionStore(storage,"test");
  const wrong: Response={status:"ok",replayed:true,result:{space,names:["Board"],state:"trashed",deletion_groups:[],empty:true,version:Array(32).fill(9)}};
  expect(validateResponse(wrong,"column.trash")).toBe(false);
  expect((await new CommandClient(restarted,async()=>wrong).deliver(id)).status).toBe("pending");
  const result={space,column,names:["Ёж 😀"],positions:[0],state:"trashed" as const,deletion_groups:["original group"],empty:true,version:Array(32).fill(9)};
  const response: Response={status:"ok",replayed:true,result};
  expect(validateResponse(response,"column.trash")).toBe(true);
  expect(validateResponse(response,"column.list")).toBe(false);
  expect(validateResponse({...response,result:{columns:[result]}},"column.list")).toBe(true);
  expect(validateResponse({...response,result:{...result,positions:["0"]}},"column.trash")).toBe(false);
  expect(validateResponse({...response,result:{...result,owner:true}},"column.trash")).toBe(false);
  expect((await new CommandClient(restarted,async intention=>{expect(intention.body).toBe(saved);return response;}).deliver(id)).status).toBe("completed");
  expect(JSON.parse(saved).parameters.expected[0]).toBe(7);
});

test("space creation preserves its target and exact Unicode name across lost replies and client restart", async () => {
  const {store, storage, values}=fixture();
  const space=newSpaceId();
  expect(space).toMatch(/^[a-f0-9]{32}$/);expect(newSpaceId()).not.toBe(space);
  const params={space,name:"Ёж 空間 😀\nnotes"};
  const id=await prepare("2".repeat(32),"space.create",params,store);
  const saved=(await store.load(id))!.body;
  params.name="Unsent later input";
  const sent: string[]=[];
  expect((await new CommandClient(store,async intention=>{sent.push(intention.body);throw new Error("lost response");}).deliver(id)).status).toBe("pending");
  const restored=webIntentionStore(storage,"test");
  expect((await restored.load(id))!.body).toBe(saved);
  const response: Response={status:"ok",replayed:true,result:{space,names:["Ёж 空間 😀\nnotes"],state:"active",deletion_groups:[],empty:true,version:Array(32).fill(9)}};
  const client=new CommandClient(restored,ipcTransport(async (_,args)=>{
    sent.push((args.invocation as {request:string}).request);
    return JSON.stringify(response);
  },async()=>({subject:"2".repeat(32),key:"approved"})));
  expect((await client.deliver(id)).status).toBe("completed");
  expect(sent).toHaveLength(4);expect(sent.every(body=>body===saved)).toBe(true);
  expect(JSON.parse(saved).parameters.space).toBe(space);
  expect(JSON.parse(saved).parameters.name).toBe("Ёж 空間 😀\nnotes");
  expect([...values.values()].join()).not.toContain("approved");
});

test("a valid response for another operation cannot confirm a space creation", async () => {
  const {store}=fixture();
  const space=newSpaceId();
  const id=await prepare("2".repeat(32),"space.create",{space,name:"One"},store);
  expect(validateResponse(success,"access.inspect")).toBe(true);
  expect(validateResponse(success,"space.create")).toBe(false);
  expect((await new CommandClient(store,async()=>success).deliver(id)).status).toBe("pending");
  expect(await store.load(id)).toBeDefined();
  const response: Response={status:"ok",replayed:true,result:{space,names:["One"],state:"active",deletion_groups:[],empty:true,version:Array(32).fill(1)}};
  expect(validateResponse(response,"space.create")).toBe(true);
  expect(validateResponse(response,"space.list")).toBe(false);
  expect(validateResponse({...response,result:{spaces:[response.result]}},"space.list")).toBe(true);
  expect(validateResponse({...response,result:{...response.result,state:"deleted"}},"space.create")).toBe(false);
  expect(validateResponse({...response,result:{...response.result,owner:true}},"space.create")).toBe(false);
  expect((await new CommandClient(store,async()=>response).deliver(id)).status).toBe("completed");
});

test("combined administration and exact spaces use the Rust-derived validator and normal delivery", async () => {
  const response: Response = { ...success, result: { ...success.result,
    scope: { kind: "administration_and_spaces", spaces: { a: "read_write", b: "read" } },
  } };
  expect(validateResponse(response)).toBe(true);
  for (const scope of [
    {kind:"administration_and_spaces"},
    {kind:"administration_and_spaces",spaces:{a:"owner"}},
    {kind:"administration_and_spaces",spaces:{a:"read"},create_spaces:true},
    {kind:"administration",spaces:{a:"read_write"}},
  ]) expect(validateResponse({...response, result:{...response.result,scope}})).toBe(false);
  const { store } = fixture();
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  expect((await new CommandClient(store, async () => response).deliver(id)).status).toBe("completed");
  expect(await store.list()).toEqual([]);
});

test("fresh cryptographic intentions preserve Unicode, original versions, and independent IDs", async () => {
  const { store } = fixture();
  const params = { subject: "空間", expected: Array(32).fill(7) };
  const id = await prepare("2".repeat(32), "access.revoke", params, store);
  const other = await prepare("2".repeat(32), "access.revoke", params, store);
  expect(id).toMatch(/^[a-f0-9]{32}$/); expect(id).not.toBe(other);
  params.expected[0] = 9;
  const saved = (await store.load(id))!;
  expect(JSON.parse(saved.body).parameters.expected[0]).toBe(7);
  expect(JSON.parse(saved.body).parameters.subject).toBe("空間");
  expect(JSON.parse(saved.body).id).toHaveLength(16);
});

test("failed durable save prevents any delivery and stores never contain credentials", async () => {
  const { store, values } = fixture();
  const failed: IntentionStore = { ...store, async save() { throw new Error("quota"); } };
  await expect(prepare("2".repeat(32), "access.inspect", parameters, failed)).rejects.toThrow("quota");
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  const client = new CommandClient(store, ipcTransport(async (_, args) => {
    expect(args.credential).toBe("private key");
    expect([...values.values()].join()).not.toContain("private key");
    return JSON.stringify(success);
  }, async () => ({subject:"2".repeat(32),key:"private key"})));
  expect((await client.deliver(id)).status).toBe("completed");
  expect(await store.list()).toEqual([]);
});

test("lost replies and automatic attempts preserve exact HTTP bytes, ID and preconditions", async () => {
  const { store } = fixture();
  const id = await prepare("2".repeat(32), "access.revoke", { ...parameters, expected: Array(32).fill(3) }, store);
  const original = (await store.load(id))!.body;
  const sent: string[] = [];
  const fake = (async (request: Request) => {
    expect(request.url).toBe("http://127.0.0.1:9999/v1/commands/access.revoke");
    expect(request.headers.get("authorization")).toBe("Bearer secret");
    expect(request.redirect).toBe("error");
    sent.push(await request.text());
    if (sent.length === 1) throw new Error("lost reply after commit");
    return new globalThis.Response(JSON.stringify(success), { headers: { "content-type": "application/json" } });
  }) as typeof fetch;
  const result = await new CommandClient(store, httpTransport("http://127.0.0.1:9999", async () => ({subject:"2".repeat(32),key:"secret"}), fake)).deliver(id);
  expect(result.status).toBe("completed");
  expect(sent).toEqual([original, original]);
});

test("bounded failure remains pending and explicit retry after restart loads the original intention", async () => {
  const { store, storage } = fixture();
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  let attempts = 0;
  const original = (await store.load(id))!.body;
  const first = await new CommandClient(store, async () => { attempts++; throw new Error("offline"); }).deliver(id);
  expect(first.status).toBe("pending"); expect(attempts).toBe(3);
  const reopened = webIntentionStore(storage, "test");
  expect(await reopened.list()).toEqual([id]);
  const resumed = await new CommandClient(reopened, async intention => { expect(intention.body).toBe(original); return success; }).deliver(id);
  expect(resumed.status).toBe("completed");
});

test("denial, stale state and mismatch never create or automatically send a new intention", async () => {
  for (const code of ["access_denied", "stale_state", "command_mismatch", "invalid_request", "not_found"] as const) {
    const { store } = fixture();
    const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
    let attempts = 0;
    const result = await new CommandClient(store, async () => { attempts++; return {status:"error",error:{code}}; }).deliver(id);
    expect(result.status).toBe("pending"); expect(attempts).toBe(1);
    expect(await store.list()).toEqual([id]);
  }
});

test("uncertain server results and malformed replies cannot confirm completion", async () => {
  for (const response of [{status:"error",error:{code:"outcome_unknown"}}, {status:"ok"}, undefined]) {
    const { store } = fixture();
    const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
    let attempts = 0;
    const result = await new CommandClient(store, async () => { attempts++; return response as Response; }).deliver(id);
    expect(result.status).toBe("pending"); expect(attempts).toBe(3);
    expect(await store.load(id)).toBeDefined();
  }
});

test("IPC timeout does not cancel admitted work or lose its original ID", async () => {
  const { store } = fixture();
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  const sent: string[] = [];
  const client = new CommandClient(store, async intention => {
    sent.push(intention.body);
    if (sent.length === 1) return new Promise<Response>(() => {});
    return success;
  });
  expect((await client.deliver(id)).status).toBe("completed");
  expect(sent).toHaveLength(2); expect(sent[0]).toBe(sent[1]);
}, 15_000);

test("HTTP key cannot follow redirects or be embedded in endpoint URLs", () => {
  for (const endpoint of ["http://example.com", "https://user:pass@example.com", "https://example.com/?key=secret", "https://example.com/#key"]) {
    expect(() => httpTransport(endpoint, async () => ({subject:"2".repeat(32),key:"secret"}))).toThrow();
  }
});

test("damaged stored ID or operation cannot silently become another intention after restart", async () => {
  const { store, storage, values } = fixture();
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  const itemKey = [...values.keys()][0];
  const saved = JSON.parse(values.get(itemKey)!);
  const request = JSON.parse(saved.body); request.id[0] ^= 1;
  storage.setItem(itemKey, JSON.stringify({ ...saved, body: JSON.stringify(request) }));
  let deliveries = 0;
  const client = new CommandClient(store, async () => { deliveries++; return success; });
  await expect(client.deliver(id)).rejects.toThrow("invalid_intention");
  expect(deliveries).toBe(0);
});

test("switching credential identity leaves the old intention pending without delivering it as another subject", async () => {
  const { store } = fixture();
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  let invocations = 0;
  const transport = ipcTransport(async () => { invocations++; return JSON.stringify(success); },
    async () => ({subject:"3".repeat(32),key:"different approved principal"}));
  expect(await new CommandClient(store, transport).deliver(id)).toEqual({status:"pending",reason:"identity_changed"});
  expect(invocations).toBe(0);
  expect(await store.load(id)).toBeDefined();
});

test("Rust-derived runtime validation rejects malformed nested results without a parallel handwritten schema", async () => {
  expect(validateResponse(success)).toBe(true);
  for (const mutation of [
    (value: any) => { value.result = {}; },
    (value: any) => { value.result.version = [1]; },
    (value: any) => { value.result.version[0] = 256; },
    (value: any) => { value.result.revoked = "true"; },
    (value: any) => { value.result.scope.kind = "owner"; },
    (value: any) => { value.result.scope.extra = "must not be accepted"; },
    (value: any) => { value.error = {code:"outcome_unknown"}; },
    (value: any) => { value.result.secret = "must not be accepted"; },
  ]) {
    const value = structuredClone(success); mutation(value);
    expect(validateResponse(value)).toBe(false);
  }
  const { store } = fixture();
  const id = await prepare("2".repeat(32), "access.inspect", parameters, store);
  const malformed = { ...success, result:{} } as Response;
  expect((await new CommandClient(store, async () => malformed).deliver(id)).status).toBe("pending");
  expect(await store.load(id)).toBeDefined();
});
