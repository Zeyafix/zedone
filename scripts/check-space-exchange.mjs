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
