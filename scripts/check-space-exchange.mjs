import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";

export function checkSpaceExchange({ host, android, space, pairing, writable, call, command, ok, error, previous }) {
  const id = () => randomBytes(16).toString("hex");
  const ids = previous ?? { column: id(), card: id(), tag: id() };
  const inspect = target => ok(command(target, "sync.inspect", { space, pairing }));
  const select = (target, selected) => { const c=inspect(target); return ok(command(target,"sync.select",{space,pairing,selected,expected:c.selection_version})); };
  const pause = (target, paused) => { const c=inspect(target); return ok(command(target,"sync.pause",{space,pairing,paused,expected:c.pause_version})); };
  if (!previous) {
    ok(command(host,"column.create",{space,column:ids.column,name:"Exchange column"}));
    ok(command(host,"card.create",{space,column:ids.column,card:ids.card,title:"Exchange card",description:"Saved locally before exchange"}));
    ok(command(host,"tag.create",{space,tag:ids.tag,name:"Exchange tag"}));
    const card=ok(command(host,"card.read",{space,card:ids.card}));
    const tag=ok(command(host,"tag.read",{space,tag:ids.tag}));
    ok(command(host,"card.set_tag",{space,card:ids.card,tag:ids.tag,present:true,expected:card.version,expected_tag:tag.version}));
  }
  assert.equal(inspect(android).selected,false,"New pairing inherited exchange selection");
  select(host,true);select(android,true);
  const outcome=ok(call(android,"exchange.run",{space}));
  assert.equal(outcome.state,writable?"up_to_date":"read_only");
  const local=ok(command(android,"card.read",{space,card:ids.card}));
  assert.deepEqual(local.titles,["Exchange card"]);assert.deepEqual(local.descriptions,["Saved locally before exchange"]);assert.deepEqual(local.tags,[ids.tag]);
  const history=ok(command(android,"card.history",{space,card:ids.card,after:null,limit:50,expected:null}));
  assert.ok(history.versions.length>0);
  const edited=ok(command(android,"card.set_completed",{space,card:ids.card,completed:true,expected:local.version}));
  ok(call(android,"exchange.run",{space}));
  assert.deepEqual(ok(command(host,"card.read",{space,card:ids.card})).completed,[writable]);
  ok(command(android,"card.set_completed",{space,card:ids.card,completed:false,expected:edited.version}));
  pause(android,true);
  error(call(android,"exchange.run",{space}),"conflict");
  select(android,false);select(android,true);assert.equal(inspect(android).paused,true);
  pause(android,false);pause(host,true);
  assert.equal(ok(call(android,"exchange.run",{space})).state,"remote_paused");
  pause(host,false);ok(call(android,"exchange.run",{space}));
  assert.deepEqual(ok(command(host,"card.read",{space,card:ids.card})).completed,[false]);
  assert.deepEqual(ok(command(android,"card.history",{space,card:ids.card,after:null,limit:50,expected:null})).versions,history.versions,"Delivery duplicated history");
  const off=select(android,false);assert.equal(off.selected,false);
  assert.deepEqual(ok(command(android,"card.read",{space,card:ids.card})).titles,["Exchange card"]);
  select(android,true);
  return ids;
}

export function checkTextExchange({ host, android, space, pairing, call, command, ok, error }) {
  const card = randomBytes(16).toString("hex");
  const column = ok(command(host,"column.list",{space,include_trashed:false})).columns[0].column;
  const read = target => ok(command(target,"card.read",{space,card}));
  const history = target => ok(command(target,"card.history",{space,card,after:null,limit:100,expected:null})).versions;
  const pause = paused => { const c=ok(command(android,"sync.inspect",{space,pairing})); ok(command(android,"sync.pause",{space,pairing,paused,expected:c.pause_version})); };
  const exchange = () => ok(call(android,"exchange.run",{space}));
  ok(command(host,"card.create",{space,column,card,title:"Text base",description:"Base description"}));exchange();pause(true);
  for (const [target,title,description] of [[host,"PC title","Base description"],[android,"Text base","Android description"]]) {
    ok(command(target,"card.edit",{space,card,title,description,expected:read(target).version}));
  }
  pause(false);exchange();
  assert.deepEqual(read(host).titles,["PC title"]);assert.deepEqual(read(host).descriptions,["Android description"]);
  assert.deepEqual(read(host),read(android));pause(true);
  for (const [target,title,description] of [[host,"PC variant","PC body"],[android,"Android variant","Android body"]]) {
    ok(command(target,"card.edit",{space,card,title,description,expected:read(target).version}));
  }
  pause(false);exchange();
  assert.deepEqual(read(host).titles,["Android variant","PC variant"]);
  assert.deepEqual(read(host).descriptions,["Android body","PC body"]);
  assert.equal(read(android).state,"conflicted");
  error(command(android,"card.set_completed",{space,card,completed:true,expected:read(android).version}),"conflict");
  pause(true);
  for (const [target,title] of [[host,"PC choice"],[android,"Android choice"]]) {
    ok(command(target,"card.resolve_text",{space,card,title,description:"Merged body",expected:read(target).version}));
  }
  pause(false);exchange();
  assert.deepEqual(read(host).titles,["Android choice","PC choice"]);
  assert.deepEqual(read(host).descriptions,["Merged body"]);
  assert.deepEqual(read(host),read(android));
  const before = read(android);
  ok(command(android,"card.resolve_text",{space,card,title:"Final choice 😀",description:"",expected:before.version}));exchange();exchange();
  assert.equal(read(host).state,"active");assert.deepEqual(read(host),read(android));
  assert.deepEqual(read(host).titles,["Final choice 😀"]);assert.deepEqual(read(host).descriptions,[""]);
  assert.deepEqual(history(host),history(android));assert.equal(history(host).length,8);
  const original = history(host)[0];
  ok(command(host,"card.restore_text",{space,card,text_version:original.id,expected:read(host).version}));exchange();
  assert.deepEqual(read(android).titles,["Text base"]);assert.deepEqual(history(host),history(android));
  assert.equal(history(android).length,9);
  console.log("Actual APK text exchange: different fields/concurrent variants/independent resolutions/history/reverse restore passed.");
}
